# PokeCrystal CLI Skill

Use `pokecrystal-cli` to run the local MCP proxy, register player identities, and hand-play Pokemon Crystal through the Ink terminal UI.

## Commands

- `pokecrystal-cli mcp` starts the stdio MCP proxy.
- `pokecrystal-cli play` starts a local streamable HTTP MCP server and opens the Ink TUI.
- `pokecrystal-cli play-recorded` opens the same TUI with training capture defaults for recorded play.
- `pokecrystal-cli register` bootstraps an identity token and session secret.
- `pokecrystal-cli skill --print` prints this skill body.

## Common Options

- `--transport local|http` selects local in-process runtime or a remote app. HTTP mode also needs `--base-url <url>`.
- `--session-id <id>` keeps the game session stable. `play` defaults to `cli-play`; use an explicit id for deliberate resumes.
- `--training-dir <path>` chooses where `play` and `play-recorded` write training capture.
- `--no-record-training` disables default TUI training capture.
- `--token <token>` and `--session-secret <value>` authenticate protected HTTP routes.

## Linked Agent Play

Build `@pokecrystal/cli` and `@pokecrystal/agents`, then start `play` with `--agent`. The TUI starts a local MCP endpoint and spawns `packages/agents/dist/bin/pokecrystal-agents.js`.

Use `codex/gpt-5.5` for GPT-5.5 gameplay through the native Codex harness:

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

Resume the same shell run later with the same `--session-id` and `--training-dir`, plus `--agent-command resume`.

For local OpenAI-compatible servers such as llama.cpp, use the `ollama/<model-id>` route and set `LLAMA_CPP_BASE_URL`. The model id after `ollama/` must match the id returned by `/v1/models`.

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

Local model notes:

- `LLAMA_CPP_BASE_URL` can point at the server root or a `/v1` URL.
- `OLLAMA_API_KEY=local` is a dummy key for local servers that require an authorization value.
- For Ollama itself, use `OLLAMA_BASE_URL=http://127.0.0.1:11434` and a model such as `ollama/gemma4:26b`.
- `--agent-max-steps` defaults to infinite. Add a small value only for a bounded smoke test.

Useful TUI commands during linked-agent play:

- `:p` or an empty command: play view.
- `:a`: agent detail view.
- `:as`: split Game Boy / agent view.
- `:set`: settings view.
- `:t`: start, pause, or resume the linked agent.
- `:i <message>`: interrupt the running agent with a correction or hint.
- `:set model <name>`, `:set goal <text>`, `:set steps <n>`, `:set cycle <n>`, `:set delay <ms>`, and `:set identity <name>` update linked-agent settings.
