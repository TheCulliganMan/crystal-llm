# @pokecrystal/agents

Mastra-based gameplay agent runner for Pokemon Crystal.

## Status
- This package is publishable.
- It contains the current taskmaster/player gameplay agent stack.
- The primary runtime is the web app MCP surface plus the `pokecrystal-agents` CLI.

## What Is Included
- `pokecrystal-agents` CLI with `run` and `resume`
- Mastra workflow runner: `pokemon-crystal-taskmaster-workflow`
- Taskmaster/player split with prompt-driven gameplay
- Codex harness support for `codex/*` models
- Session auth persistence in `packages/agents/.session-auth`
- MCP toolbox access to gameplay state and actions
- Training trace compatibility with the web MCP recorder

## Model Routing
- `codex/<model>`: native Codex harness path, for example `codex/gpt-5.5`
- `openai/<model>`: Mastra OpenAI-style provider path, for example `openai/gpt-5.4`
- `ollama/<model-id>`: local OpenAI-compatible endpoints such as `llama.cpp`

## Command Reference

```bash
pokecrystal-agents run \
  --session-id <id> \
  --model <provider/model> \
  --mcp-base-url <url> \
  --graph-cycle-steps <n> \
  --goal "<text>" \
  --identity-name <name>
```

Use `run` to start autonomous gameplay against a web app MCP base URL. Use `resume` with the same flags when the same workflow should receive a new goal or continue after a prior batch. `--max-steps <n>` is an optional batch cap; omit it for an infinite run.

For a TUI-owned MCP server, pass the exact MCP URL printed by `pokecrystal-cli play`:

```bash
pokecrystal-agents run \
  --session-id <id> \
  --model codex/gpt-5.5 \
  --mcp-url http://127.0.0.1:<port>/mcp?session_id=<id> \
  --graph-cycle-steps 800000 \
  --request-delay-ms 100 \
  --goal "Get the starter Pokemon, then continue toward beating Mt. Silver" \
  --identity-name codex-gpt-5-5-agent
```

For `ollama/*` models backed by llama.cpp, set `LLAMA_CPP_BASE_URL` and `OLLAMA_API_KEY`:

```bash
env LLAMA_CPP_BASE_URL=http://127.0.0.1:8080 \
  OLLAMA_API_KEY=local \
  pokecrystal-agents run \
  --session-id local-llamacpp-gemma4-e2b \
  --model ollama/gemma-4-E4B-it-Q4_K_M.gguf \
  --mcp-url http://127.0.0.1:<port>/mcp?session_id=local-llamacpp-gemma4-e2b \
  --graph-cycle-steps 800000 \
  --request-delay-ms 100 \
  --goal "Get the starter Pokemon, then continue toward beating Mt. Silver" \
  --identity-name llamacpp-gemma4-agent-2
```

## Session Model
- Start a session once with `run`.
- Keep the same session alive with `resume`.
- Do not rotate session ids if you want honest continuity.
- The web app records MCP-side training data under `apps/web/.pokecrystal-agents/runs/<session-id>/training`.

## Live Inspection
- Runtime snapshot: `apps/web/mcp-<session-id>-runtime.json`
- Episode trace: `apps/web/.pokecrystal-agents/runs/<session-id>/training/episode.jsonl`
- Session auth cache: `packages/agents/.session-auth/<session-id>.json`

## Development Commands

```bash
npm run build --workspace @pokecrystal/agents
npm run lint:types --workspace @pokecrystal/agents
npm run test --workspace @pokecrystal/agents
```

## Release Notes
- Package entrypoint: `@pokecrystal/agents`
- Executable: `pokecrystal-agents`
- Published files include `dist`, `skills`, and this README
