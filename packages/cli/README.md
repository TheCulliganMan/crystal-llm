# @pokecrystal/cli

CLI, stdio MCP server, and terminal UI for the PokeCrystal runtime.

## Status
- This is a publishable package.
- By default the CLI runs the game locally in-process.
- To target a running web app instead, pass `--transport http --base-url http://localhost:3000`.

## Primary Surfaces
- Stdio MCP server: `pokecrystal-cli mcp`
- Terminal text UI: `pokecrystal-cli play`
- Recorded terminal text UI: `pokecrystal-cli play-recorded`
- Skill document export: `pokecrystal-cli skill --print`

## Command Reference

| Command | Purpose |
| --- | --- |
| `pokecrystal-cli mcp` | Starts a stdio MCP proxy for the local runtime, or for a remote app when `--transport http --base-url <url>` is set. |
| `pokecrystal-cli play` | Starts the local Ink TUI, a local runtime, and a TUI-owned streamable HTTP MCP endpoint for the same session. |
| `pokecrystal-cli play-recorded` | Starts the same TUI with recorded-play training defaults under `packages/cli/.tmp-human-play/<session-id>`. |
| `pokecrystal-cli register` | Registers an identity and prints the session secret needed for protected HTTP routes. |
| `pokecrystal-cli skill` | Prints the packaged Codex skill path, or the skill body with `--print`. |

Common options:

| Option | Applies To | Notes |
| --- | --- | --- |
| `--transport local\|http` | `mcp`, `play`, `play-recorded`, `register` | Defaults to `local` unless a base URL is configured. |
| `--base-url <url>` | `mcp`, `play`, `play-recorded`, `register` | App origin for HTTP transport, for example `http://localhost:3000`. A `/api/mcp/tools` suffix is accepted and normalized. |
| `--session-id <id>` | All runtime commands | Stable session id. `play` and `play-recorded` default to `cli-play` when omitted; use an explicit id for deliberate resumes. |
| `--token <token>` | Runtime commands | Identity bearer token for protected routes. |
| `--session-secret <value>` | Runtime commands | Session secret for protected routes. |
| `--training-dir <path>` | `play`, `play-recorded` | Overrides the training capture directory. |
| `--no-record-training` | `play`, `play-recorded` | Disables the default training capture for TUI play. |
| `--agent-id <id>` | `register` | Optional identity registration agent id. |
| `--identity-name <name>` | `register` | Optional identity registration display name. |

Linked-agent options for `play` and `play-recorded`:

| Option | Notes |
| --- | --- |
| `--agent` | Spawns `packages/agents/dist/bin/pokecrystal-agents.js` and links it to the TUI-owned MCP endpoint. Build `@pokecrystal/agents` first. |
| `--agent-command run\|resume` | Starts a new agent run or resumes an existing one. Default: `run`. |
| `--agent-model <provider/model>` | Agent model, for example `codex/gpt-5.5` or `ollama/<model-id>`. |
| `--agent-goal <text>` | Goal prompt passed to the linked agent. |
| `--agent-max-steps <n>` | Optional maximum supervised gameplay batches for the linked agent process. Omit it for an infinite run. |
| `--agent-graph-cycle-steps <n>` | Agent graph cycle budget per batch. |
| `--agent-request-delay-ms <n>` | Delay between agent batches. |
| `--agent-identity-name <name>` | Identity name used by the linked agent. |

Agent model routes:

| Model value | Route | Notes |
| --- | --- | --- |
| `codex/gpt-5.5` | Native Codex harness | Use this for GPT-5.5 gameplay through the Codex runner. No local model server URL is required. |
| `openai/<model>` | Mastra OpenAI provider | Uses the normal OpenAI-compatible provider path and `OPENAI_API_KEY`/`OPENAI_BASE_URL` when configured for the agent runner. |
| `ollama/<model-id>` | Local OpenAI-compatible server | Use this for Ollama or llama.cpp-compatible servers. Set `LLAMA_CPP_BASE_URL` or `OLLAMA_BASE_URL`, plus `OLLAMA_API_KEY`. |

## Command Examples

### MCP Over Stdio

```bash
pokecrystal-cli mcp --session-id <uuid>
```

Starts a real MCP server over stdio against the local runtime.

### Terminal Text UI

```bash
pokecrystal-cli play --session-id <uuid>
```

From a workspace checkout, build the package first and run the compiled binary:

```bash
npm run build --workspace @pokecrystal/cli
node packages/cli/dist/bin/pokecrystal-cli.js play --session-id my-session
```

`play` starts a local in-process runtime by default, opens the Ink TUI, and creates a streamable HTTP MCP endpoint for the same session. The TUI header prints the endpoint, for example:

```text
MCP: http://127.0.0.1:<port>/mcp?session_id=my-session
```

To drive a running web app instead of the local runtime, pass HTTP transport:

```bash
node packages/cli/dist/bin/pokecrystal-cli.js play \
  --transport http \
  --base-url http://localhost:3000 \
  --session-id my-session
```

If `--session-id` is omitted, `play` uses the stable `cli-play` session. Training capture is enabled by default and writes to `./.pokecrystal-cli/runs/<session-id>/training`; pass `--no-record-training` to disable it or `--training-dir <path>` to choose a different output directory.

### Terminal Text UI With GPT-5.5

Build both packages before using `--agent`; the TUI spawns the compiled agent runner and connects it to the same local MCP endpoint that the TUI is using.

```bash
npm run build:cli
npm run build:agents

node packages/cli/dist/bin/pokecrystal-cli.js play \
  --session-id local-codex-mt-silver \
  --agent \
  --agent-model codex/gpt-5.5 \
  --agent-goal "Get the starter Pokemon, then continue toward beating Mt. Silver" \
  --agent-graph-cycle-steps 800000 \
  --agent-request-delay-ms 100 \
  --agent-identity-name codex-gpt-5-5-agent-2 \
  --training-dir /tmp/pokecrystal-local-codex-mt-silver
```

Use `codex/gpt-5.5` when you want GPT-5.5 to make gameplay decisions through the native Codex harness. The TUI still accepts normal keyboard input, so you can take over manually and then resume the agent without changing commands.

For a later shell restart of the same run, keep the same `--session-id` and `--training-dir` and add `--agent-command resume`:

```bash
node packages/cli/dist/bin/pokecrystal-cli.js play \
  --session-id local-codex-mt-silver \
  --agent \
  --agent-command resume \
  --agent-model codex/gpt-5.5 \
  --agent-goal "Get the starter Pokemon, then continue toward beating Mt. Silver" \
  --agent-graph-cycle-steps 800000 \
  --agent-request-delay-ms 100 \
  --agent-identity-name codex-gpt-5-5-agent-2 \
  --training-dir /tmp/pokecrystal-local-codex-mt-silver
```

Use a new `--session-id` when you want a clean game session and separate training trace.

### Terminal Text UI With a Local llama.cpp-Compatible Agent

The agent uses the `ollama/*` model route for local OpenAI-compatible servers. With `LLAMA_CPP_BASE_URL`, the model name after `ollama/` must match the id returned by the server's `/v1/models` endpoint.

First make sure the local server is running and exposes OpenAI-compatible models:

```bash
curl -s http://127.0.0.1:8080/v1/models
```

Then start the TUI and linked agent:

```bash
env LLAMA_CPP_BASE_URL=http://127.0.0.1:8080 \
  OLLAMA_API_KEY=local \
  node packages/cli/dist/bin/pokecrystal-cli.js play \
  --session-id local-llamacpp-gemma4-e2b \
  --agent \
  --agent-model ollama/gemma-4-E4B-it-Q4_K_M.gguf \
  --agent-goal "Get the starter Pokemon, then continue toward beating Mt. Silver" \
  --agent-graph-cycle-steps 800000 \
  --agent-request-delay-ms 100 \
  --agent-identity-name llamacpp-gemma4-agent-2 \
  --training-dir /tmp/pokecrystal-local-llamacpp-gemma4
```

Notes:

- `LLAMA_CPP_BASE_URL` can be either the server root, such as `http://127.0.0.1:8080`, or a `/v1` URL. The agent normalizes it before calling chat completions.
- `OLLAMA_API_KEY=local` is a dummy key for local servers that still require an authorization value.
- `--agent-model ollama/gemma-4-E4B-it-Q4_K_M.gguf` means the local model id must be `gemma-4-E4B-it-Q4_K_M.gguf`. Change only the part after `ollama/` when your `/v1/models` response reports a different id.
- For Ollama itself, use `OLLAMA_BASE_URL=http://127.0.0.1:11434` and a model value such as `ollama/gemma4:26b`.
- `--agent-max-steps` is optional and defaults to an infinite run. Add a small value only when you want a bounded smoke test.

### Recorded Terminal Text UI

```bash
pokecrystal-cli play-recorded --session-id <session-id>
```

This launches the same local text UI, enables training capture by default, and writes run data to `packages/cli/.tmp-human-play/<session-id>` unless `--training-dir` is provided.

Controls:
- Arrow keys, `WASD`, or `HJKL`: d-pad
- `Z`, `J`, `Space`, or `Shift+A`: A
- `X`, `K`, or `B`: B
- `Enter`: Start
- `Tab`: Select
- `.`: wait 8 frames
- `R`: refresh
- `:v`: cycle views: play, agent detail, split Game Boy / agent, and settings
- `:t`: start, pause, or resume the linked agent
- `:i <message>`: interrupt the running agent with a correction or hint
- `:set model <name>`: switch the linked agent model
- `:set goal <text>`: update the linked agent goal
- `:set steps <n>`, `:set cycle <n>`, `:set delay <ms>`, or `:set identity <name>`: update linked agent settings
- `:a`: toggle TUI audio
- `:u`: toggle the current Game Boy panel between text and Kitty/Ghostty image rendering
- `Esc` or `:`: command mode
- `:q!`: quit without saving
- `:wq`, `:wq!`, `:x`, or `:x!`: save-compatible quit command aliases

TUI audio is played by a local command-line audio player, not by terminal escape sequences. On macOS the CLI uses `afplay`; on Linux it tries common players such as `mpg123`, `mpv`, `ffplay`, `play`, and `paplay`. Ghostty does not need a special setting for game music or SFX, though its own BEL alert sound is controlled separately by Ghostty's `bell-features` config.

### Identity Bootstrap

```bash
pokecrystal-cli register \
  --session-id <uuid> \
  --identity-name trainer-oak
```

### Skill Output

```bash
pokecrystal-cli skill --print
```

## Environment Variables
- `POKECRYSTAL_BASE_URL` or `POKECRYSTAL_TOOLS_BASE_URL`: default app URL for HTTP transport.
- `POKECRYSTAL_CLI_TRANSPORT`: default transport, `local` or `http`.
- `POKECRYSTAL_SESSION_ID`: default session id.
- `POKECRYSTAL_IDENTITY_TOKEN`: default identity bearer token.
- `POKECRYSTAL_SESSION_SECRET`: default protected-route session secret.
- `POKECRYSTAL_CLI_RECORD_TRAINING`: set `0` to disable default `play` training capture, or `1` to force capture for commands that do not enable it by default.
- `POKECRYSTAL_CLI_TRAINING_DIR`: default training output directory.
- `POKECRYSTAL_CLI_AGENT`: set `1` to enable `--agent` for TUI play.
- `POKECRYSTAL_AGENT_COMMAND`: linked agent command, `run` or `resume`.
- `POKECRYSTAL_AGENT_MODEL`: default linked agent model.
- `POKECRYSTAL_AGENT_GOAL`: default linked agent goal.
- `POKECRYSTAL_AGENT_MAX_STEPS`: optional linked agent max-step cap. Omit it for infinite.
- `POKECRYSTAL_AGENT_GRAPH_CYCLE_STEPS`: default linked agent graph cycle budget.
- `POKECRYSTAL_AGENT_REQUEST_DELAY_MS`: default delay between linked agent batches.
- `POKECRYSTAL_AGENT_IDENTITY_NAME`: default linked agent identity name.
- `POKECRYSTAL_CLI_AUDIO_PLAYER`: override the local audio player command. Use `{file}` where the audio path should be inserted, for example `ffplay -nodisp -autoexit -loglevel quiet {file}`.
- `POKECRYSTAL_CLI_AUDIO_ROOT`: override the root directory used to resolve `/api/audio/...` files.
- `LLAMA_CPP_BASE_URL` or `OLLAMA_BASE_URL`: local OpenAI-compatible base URL for `ollama/*` agent models.
- `OLLAMA_API_KEY`: dummy or real API key for local Ollama-compatible model adapters.

## Development Commands

```bash
npm run build --workspace @pokecrystal/cli
npm run lint:types --workspace @pokecrystal/cli
npm run test --workspace @pokecrystal/cli -- --runInBand
```

The package includes executable end-to-end coverage for the `mcp`, `play`, `register`, and `skill` surfaces.

## Release Notes
- Package name: `@pokecrystal/cli`
- Executable: `pokecrystal-cli`
- Published files include `dist`, `server.json`, `skills`, and this README
