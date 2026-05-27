---
name: crystal-llm
description: Use the local crystal-llm Pokemon Crystal MCP over streamable HTTP on this machine. Use when Codex needs to play, inspect, debug, or automate Pokemon Crystal through the persistent localhost MCP session, capture screenshots, update route-learning memory, or diagnose direct-HTTP harness issues.
---

# Crystal LLM

Use the persistent local MCP session for Pokemon Crystal.

- Endpoint: `http://127.0.0.1:3110/api/mcp?session_id=codex-service`
- Continuity save: `$POKECRYSTAL_REPO/apps/web/mcp-codex-service-autosave.sav`
- Server: local `@pokecrystal/web` Next app on port `3110`
- Shortcut: `poke` is installed at `$HOME/.local/bin/poke` and wraps `$CODEX_HOME/skills/crystal-llm/scripts/poke.mjs`
- Bundled command script: this skill includes `scripts/poke.mjs` for installs, repo packaging, and downloadable zips.
- Codex memory: `$CODEX_HOME/pokecrystal/`
- Learning logs: `$CODEX_HOME/pokecrystal/learnings/`

## Required Start

1. Start or check the local web MCP server:

```bash
cd $POKECRYSTAL_REPO
PORT=3110 npm run dev --workspace @pokecrystal/web
```

2. Use the stock compact tools first. They wrap direct HTTP, save screenshots, and return small JSON:

```bash
poke status
poke observe
poke route
poke route --tiles
poke move left
poke context
```

Use `scripts/mcp_call.mjs` when a raw MCP tool call is needed. Direct automation uses these Crystal LLM direct-HTTP scripts, not `mcporter`.

3. Use `mcporter` only for explicit harness diagnostics when a Codex-local config is available, not for scheduled gameplay. If `mcporter` says `Unknown MCP server 'pokecrystal'`, use the direct HTTP scripts; do not infer that the game/MCP is down.

## Reference Files

- Read [references/mcp-harness.md](references/mcp-harness.md) for service management, direct HTTP calls, screenshot capture, `mcporter` config, and harness failure triage.
- Read [references/play-policy.md](references/play-policy.md) before substantial gameplay, route recovery, training, or memory updates.
- Read [references/ryan-blog.md](references/ryan-blog.md) before touching scheduled play/posting, battle play-by-play, fan engagement, or public journal text.

## Live Play Loop

Use this loop for manual and autonomous play:

1. Call `poke status`.
2. If in dialogue, menus, battle, or a naming screen, clear or handle that UI before pathing.
3. Call `poke observe` before terrain decisions.
4. Move one tile at a time near trees, ledges, fences, grass pockets, NPCs, or route branches.
5. If the same move fails twice, treat it as a model/tactic failure; observe, identify why the tactic was bad, and choose a different lane.
6. Use `recent_events` after surprising movement, map transitions, or tool errors.
7. Write the lesson if a route assumption, UI trap, or harness failure mattered.

Do not use repo files, ROM data, save internals, emulator memory, implementation coordinates, or generated map data to decide gameplay. For gameplay, use only live MCP outputs, real screenshots, visible text, recent in-game events, controller inputs, and Ryan's directions. File access is allowed for skill/runner/memory maintenance, not for in-game navigation.

## Resource And Catching Policy

- Treat catching Pokemon as useful preparation for beating the game, not as a side objective that overrides story progress.
- Catch 'em all posture: when resources and safety allow, try to catch new species, reasonable dupes with useful roles, and Pokemon that improve party depth, type coverage, HM utility, or backup strength.
- When visiting towns, use marts more proactively. Buy more Potions and Poke Balls or better balls than the old conservative policy would, while preserving enough money for emergency survival.
- Prefer leaving town with enough Balls to make multiple capture attempts and enough Potions to extend routes without constant Pokemon Center retreats.
- When the game offers a nickname prompt after receiving or catching a Pokemon, prefer giving that Pokemon a short nickname unless doing so would block urgent progress.

## Battle And Training Policy

- Be more gung ho in battle. The current team has been underleveled, so default toward fighting ordinary wild Pokemon during travel instead of dodging every encounter.
- Use wild grass deliberately for experience when the team is behind the area, the next trainer/gym looks risky, or recent fights were close. Short training loops are useful progress, not a stall, when they produce EXP.
- Heal less often. Do not walk back to a Pokemon Center merely because HP is not full. Keep moving or training while the active Pokemon has comfortable HP, usable PP, and no dangerous status problem.
- Use items, switches, or party depth before retreating all the way to a Center when that keeps the run moving safely.
- Retreat or use a Pokemon Center when there is a concrete survival reason: red HP, repeated near-KOs, badly depleted PP, poison/burn/sleep creating travel risk, multiple fainted Pokemon, or an important upcoming fight.
- Avoid reckless loss loops. Do not chain red-HP encounters unless Ryan has explicitly allowed whiteout training or there is a deliberate reset/whiteout plan.

## Navigation Rules

- Treat Ryan's directions as route intent, not blind button macros.
- Follow visible lanes and shelves. Destination direction may require moving away from the destination first.
- Treat `d`, `l`, and `r` ledge glyphs as one-way terrain. Cross them only deliberately and only from the valid side.
- Prefer reversible exploration. Keep a route back to the current lane.
- On multi-floor or multi-warp maps, maintain a navigation ledger before moving: current map/coords, current objective, intended next warp/landmark, last useful warp used, local failed moves, and a recovery move if the next lane fails.
- Treat `route --no-image` as the floor-level warp graph when the viewport is too local. Identify the next target warp or landmark from the live render, then move through visible lanes toward it in small verified chunks.
- A blocked tile only invalidates that immediate tile, not the floor objective. After two blocked moves in a pocket, observe, call `route --no-image`, return to the last useful warp or choose a different visible lane, and continue unless HP, battle, or UI state makes movement unsafe.
- When an NPC or trainer blocks a direct line to a target warp, interact or fight if appropriate; if dialogue only or still blocked, route around using adjacent visible lanes instead of ending the run.
- Do not claim a route is impossible from a single viewport; say "not found in the observed viewport" unless a larger scouted chart proves it.
- Capture start/end proof images when Ryan asks for screenshots or when a run is being used as evidence.

## Stock Tools

Prefer the `poke` shortcut for these compact wrappers to reduce prompt and output tokens. If `poke` is not on `PATH`, use the bundled `scripts/poke.mjs` path from this skill installation, or the local full path `$CODEX_HOME/skills/crystal-llm/scripts/poke.mjs`.

```bash
poke status
poke observe
poke observe --grid
poke route
poke route --tiles
poke route --no-image
poke proof cherrygrove
poke move left
poke press A
poke clear
poke events
poke context
```

Use `context` instead of reading the full learning-state JSON when you only need the current map, next prompt, and route memory. Use `route` when the Game Boy viewport is too local for navigation; it renders the full current map without adding pathfinding or scripted movement. Add `--tiles` when a high-fidelity full-map tile image is needed instead of the schematic PNG. Use `observe --grid` only when viewport text terrain is needed; plain `observe` saves a real PNG and omits the grid to keep output small.

## Memory Contract

Route-specific corrections must live outside this reusable skill.

- Canonical state: `$CODEX_HOME/pokecrystal/poke_learning_state.json`
- Human route notes: `$CODEX_HOME/pokecrystal/poke_learning_journal.md`
- Daily sidecar: `$CODEX_HOME/pokecrystal/memory/YYYY-MM-DD-pokecrystal-learning.md`
- Durable learning log: `$CODEX_HOME/pokecrystal/learnings/LEARNINGS.md`
- Tool/runtime failures: `$CODEX_HOME/pokecrystal/learnings/ERRORS.md`

Store route-specific data under `routeMemory[mapName]`:

```json
{
  "routeMemory": {
    "<MapName>": {
      "privateNavigationHint": "Private human correction or learned route note.",
      "privateNavigationPlan": {
        "appliesWhen": { "x": [0, 999], "y": [0, 999] },
        "routeSteps": ["Observe current lane.", "Move one tile.", "If it fails twice, branch to a different tactic."],
        "inspectEvery": 5
      }
    }
  }
}
```

Do not store generic direction orders as route guidance. `routeHints` is legacy and should stay empty except during migration of old private notes into `routeMemory`.

## Autonomous Runs

Before moving, write a short run plan in durable notes or the runner summary:

- live map and current goal
- current route memory for that map
- recent failed attempts on that map
- current navigation ledger for multi-warp maps: current floor, target floor/warp/landmark, last useful warp, failed local moves, and recovery lane
- next 2-3 tactics
- survival/training policy for the active Pokemon, including whether to seek wild EXP before the next story fight

Scheduled play should do enough real interactions to matter, usually around 30. Failed movement is a model/tactic problem, not a "blocking" excuse. It should not quit after one failed movement; it should branch, interact, inspect, and keep trying unless HP, battle state, or UI state makes further play unsafe.

Codex gameplay automation must use the stock `poke` shortcut first (`status`, `observe`, `route`, `context`, `proof`, `move`, `press`, `clear`, `events`) and only use raw MCP calls for tools that wrapper does not expose. Do not route gameplay through `mcporter`.

Do not convert failed behavior into future guidance. Never write direction-order churn, scout-looping, or repeated same-tile pushing as a best practice. Only infer a stall lesson after an actual movement action fails. Diagnostic-only status/observe/context/proof runs must not create stall guidance.

## Validation

Use these checks after skill or harness changes:

```bash
python3 $CODEX_HOME/skills/.system/skill-creator/scripts/quick_validate.py $CODEX_HOME/skills/crystal-llm
node --check $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs
poke status
poke route --no-image
poke observe --save-images $CODEX_HOME/pokecrystal/mcp-images/verify
```

If Next returns `500` with `Unexpected end of JSON input`, restart the service, retry with slower one-call-at-a-time direct HTTP, and log it in `.learnings/ERRORS.md` if it affected the run.
