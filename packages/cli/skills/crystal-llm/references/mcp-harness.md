# MCP Harness

## Service

The service runs the Docker `pokecrystal-ts` service from `$POKECRYSTAL_REPO` on host port `3003`.

```bash
cd $POKECRYSTAL_REPO
docker compose up -d --build pokecrystal-ts
```

After restart, reuse `session_id=codex-service`; the session should load `$POKECRYSTAL_REPO/apps/web/mcp-codex-service-autosave.sav`.

## Stock Compact Tools

Prefer `poke.mjs` for normal play and diagnosis. It wraps direct HTTP and returns compact JSON:

```bash
node $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs status
node $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs observe
node $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs route
node $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs route --tiles
node $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs move left
node $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs press A
node $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs clear
node $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs context
```

Use `route` when the Game Boy viewport is too local for navigation. Add `--tiles` for a high-fidelity full-map tile PNG. Use `observe --grid` only when the text viewport is needed. Use `proof <label>` to save a real screenshot and return status plus image path.

## Direct HTTP Helper

Use the raw bundled helper only when a stock command is not enough:

```bash
node $CODEX_HOME/skills/crystal-llm/scripts/mcp_call.mjs status
node $CODEX_HOME/skills/crystal-llm/scripts/mcp_call.mjs list-tools
node $CODEX_HOME/skills/crystal-llm/scripts/mcp_call.mjs move --args '{"direction":"left","times":1,"steps":1,"count":1,"format":"json","detail":"compact"}'
node $CODEX_HOME/skills/crystal-llm/scripts/mcp_call.mjs observe --args '{"include_image":true,"image_scale":2,"advance_frames":1,"detail":"compact","format":"json"}' --save-images $CODEX_HOME/pokecrystal/mcp-images/manual
```

The helper talks directly to `http://127.0.0.1:3003/api/mcp?session_id=codex-service`, writes MCP image blocks as PNGs when `--save-images` is provided, and prints parsed JSON/text content.

## mcporter

Use `mcporter` only for explicit diagnostics when a Codex-local config is available. Normal Codex play should use the direct HTTP scripts above. If `mcporter` reports `Unknown MCP server 'pokecrystal'`, use the direct HTTP scripts; do not treat it as a game failure.

## Tool Usage

- `status`: first call; compact mode/map/coords/facing/UI state.
- `observe`: route view, hotspots, optional real emulator PNG.
- `move`: one directional input; include `times`, `steps`, and `count` for schema compatibility.
- `press`: button input; use for dialogue, menu, battle, and UI clearing.
- `recent_events`: verify map transitions, unexpected moves, blockers, and error context.
- `flow_state`: confirm the honest next story goal.
- `map_info`: inspect warps/hotspots without reading implementation internals.
- `route_render`: inspect the full live current map as text JSON and optional schematic or tile PNG without adding pathfinding or scripted movement.

## Harness Failure Triage

- `Unknown MCP server 'pokecrystal'`: current working directory/config problem. Fix with `--config` or use direct HTTP.
- Next `500` with `Unexpected end of JSON input`: web/dev harness failure, seen during fast sequential calls. Restart the systemd service, retry slower, and log if it affected progress.
- Black or fake screenshots: regression. Real `observe(include_image:true)` images must be emulator renders, not synthesized viewport grids.
- Long movement macros around ledges: control-policy failure. Switch to one-tile moves and observe after blockers.

When proving a harness issue, capture all three: successful direct HTTP status/observe, the failing harness command/error, and a real screenshot or map transition showing the game state.
