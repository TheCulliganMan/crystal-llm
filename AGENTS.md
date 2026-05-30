# Repository Guidelines

## Project Structure & Module Organization

This is a self-contained npm workspace TypeScript monorepo. Core game logic lives in `packages/core/src`, ASM-derived content in `packages/assets/src`, exporters in `packages/exporters/src`, CLI/MCP/TUI code in `packages/cli/src`, and agent tooling in `packages/agents/src`. The Next.js app is in `apps/web/src`; Electron packaging is in `apps/electron`. Tests are colocated as `*.test.ts` or `*.test.tsx`, with CLI e2e tests in `packages/cli/src/e2e`.

## Build, Test, and Development Commands

- `npm install`: install dependencies with Node.js `24.x` and npm `>=10.5.0`.
- `npm run dev`: start the local Next.js app.
- `npm run build`: typecheck, then build CLI and web.
- `npm run test`: run all Jest workspace tests.
- `npm run lint`: run web ESLint.
- `npm run test --workspace @pokecrystal/core`: run targeted core tests.
- `npm run export:core`: regenerate runtime data through the TypeScript exporter.

## Docker Server Commands

- `docker compose up --build pokecrystal-ts`: build and start the Docker dev server in the foreground.
- `docker compose up -d --build pokecrystal-ts`: build and start the Docker dev server in the background.
- `docker compose stop pokecrystal-ts`: stop the server container without deleting saves.
- `docker compose start pokecrystal-ts`: start the stopped server container again.
- `docker compose down`: stop and remove the container/network while keeping the named save volume.
- `docker compose down -v`: stop the stack and delete the `pokecrystal_saves` volume for a clean Docker save state.
- `docker compose logs -f pokecrystal-ts`: follow server logs.

The Docker service publishes the web/MCP server at `http://localhost:3003`.

## Coding Style & Naming Conventions

Follow existing file-local formatting; do not reformat unrelated code. TypeScript is strict via `tsconfig.base.json`. Use typed public boundaries, `camelCase` variables/functions, `PascalCase` classes and React components, and kebab-case filenames such as `battle-logic.test.ts`. Prefer workspace imports like `@pokecrystal/core` across package boundaries.

## Testing Guidelines

Use Jest for unit and integration coverage. Add or update colocated `*.test.ts` or `*.test.tsx` files. For web UI or routes, run web Jest tests and consider `npm run pw:pages --workspace @pokecrystal/web`. Do not hand-edit generated data; regenerate it and test the exporter.

## Runtime Log Lookup

CLI play sessions write JSONL logs under `/tmp` using the session id, for example `/tmp/pokecrystal-<session-id>.jsonl`; on macOS `/tmp` is a symlink to `/private/tmp`. When debugging a play crash, first list recent logs with `ls -lt /tmp/pokecrystal-*.jsonl | head`, then search the relevant file with `rg -n "session_error|<error text>|<map or script>" /tmp/pokecrystal-<session-id>.jsonl`. The `session_start` entry includes the exact `log_file` path.

## Commit & Pull Request Guidelines

Recent history uses concise Conventional Commit-style subjects: `feat: ...`, `test: ...`, `feat(exporters): ...`. Keep commits focused and imperative. PRs should explain what changed, why it matches Pokemon Crystal behavior, and which commands verified it. Link issues and include screenshots for visible UI changes.

## Agent Rules

- Self-contained codebase: do not introduce external runtime assumptions outside the workspace.
- No legacy paths and no compatibility shims. When migrating, fully remove the old path and commit to the new path.
- No fallbacks unless the behavior exists in the ASM. If the main export fails, fix the main export instead of routing around it with suboptimal code.
- If you see an unapproved fallback, stop and fix it before building on top of it.
- When improving gameplay agents, keep the agent's instruction as playing Pokemon Crystal and making honest main-story progress. Do not secretly encode side goals such as getting a starter, reaching Elm, or any other specific milestone unless the user explicitly permits that goal.
- Gameplay-agent improvements must be general enough to beat the game, not optimized only for the immediate visible bottleneck. Avoid over-indexing on the first few minutes or a single route segment.
- Do not hard-code route names, map names, NPC names, item names, story names, or regex-driven route policies into agent logic. Prefer improving the agent's live-state context, tool surface, prompting, memory, retry discipline, and action schema.
- Keep gameplay decisions agent-only: the model should choose actions from live `status`, `observe`, `map_info`, `flow_state`, and `recent_events`. Do not add deterministic controller policies, route scripts, macro routing, or non-agentic recovery to make progress.
- Wait is never a gameplay-agent option. If any agent action schema, prompt, or tool list exposes waiting, idling, skipping, deferring, no-op, or asking for user input as an action choice, remove it and require a concrete gameplay action instead.

## Bug Fix Workflow

1. Identify the bug, understand its root cause, and reproduce it.
2. Write a test case that fails because of the bug.
3. Implement the fix in the codebase.
4. Run the focused test first, then the relevant workspace checks.
