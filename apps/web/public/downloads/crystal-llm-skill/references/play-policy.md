# Play Policy

## Inputs Allowed For Gameplay

Use only live game evidence:

- Stock `scripts/poke.mjs` commands: `status`, `observe`, `context`, `proof`, `move`, `press`, `clear`, and `events`
- Raw MCP `status`, `observe`, `map_info`, `flow_state`, and `recent_events` only when the stock wrapper does not expose what is needed
- real MCP screenshots
- visible in-game text
- controller inputs and their results
- Ryan's directions
- persistent route memory and learning logs

Do not inspect ROM/source files, map/event files, save internals, emulator memory, or generated route data to decide where to go. Maintenance tasks may read/edit files, but that information must not become gameplay navigation knowledge.

## Navigation

- Read the screen before pressing buttons.
- Treat destination direction as intent, not immediate input.
- Use one-tile moves around ledges, fences, rocks, trees, NPCs, grass pockets, and branch points.
- If the same movement fails twice, treat it as a bad model/tactic choice and branch.
- Treat `d`, `l`, and `r` as one-way ledges, not generic floor.
- Do not treat direction-order churn, repeated direction pivots, or same-tile pushing as strategy. They are failure patterns, not future guidance.
- On stalls, use this sequence: `poke.mjs status` -> `poke.mjs observe` with a real image -> identify the bad tactic -> make a different deliberate move -> re-check.
- Route memory and live map context beat inferred generic lessons. In Cherrygrove, the live goal is Mr. Pokemon + Mystery Egg, not stall recovery.
- In Cherrygrove, do not repeatedly push north after it fails. Use visible town context, move west through town toward the Route 30 approach, interact with NPCs when useful, and keep pursuing progress.
- If a route is not visible in the current viewport, say that. Do not overclaim route impossibility.
- Prefer roads/floors at low HP; use grass deliberately when training or when no route avoids it.

## Battles And Training

Experience matters. Do not avoid all wild battles by default.

Catching Pokemon is useful main-story preparation when it is safe and resources allow it. Try to catch reasonable wild Pokemon that add party depth, HM utility, type coverage, or backup strength without derailing the current flow goal. When visiting towns, consider the mart if money and bag space allow; keep a practical stock of Poke Balls or better balls while preserving enough money for survival items. When the game offers a nickname prompt after receiving or catching a Pokemon, prefer giving that Pokemon a short nickname unless doing so would block urgent progress.

- Fight reasonable wild Pokemon when the active Pokemon is healthy.
- Run, heal, or retreat when HP is unsafe.
- If Ryan explicitly allows whiteouts for training, EXP preservation can make a reset acceptable.
- Do not chain red-HP encounters without a deliberate heal/reset/whiteout decision.

## Memory Updates

Update memory when a mistake or route result will matter later:

- `pokecrystal/poke_learning_state.json`: structured route memory and next prompt.
- `pokecrystal/poke_learning_journal.md`: human-readable run plans and route notes.
- `memory/YYYY-MM-DD-pokecrystal-learning.md`: sidecar notes for dreaming/reflection.
- `.learnings/LEARNINGS.md`: durable corrections and best practices.
- `.learnings/ERRORS.md`: service, MCP, `mcporter`, or runtime failures.

Route-specific corrections belong under `routeMemory[mapName]`, not in the skill body. Public Ryan blog/progress posts must not leak private route coordinates, tool traces, or harness details.

Only infer stall lessons after an actual movement attempt failed. Diagnostic-only status/observe/context/proof runs must not write stall guidance.

## Proof Discipline

When Ryan asks for screenshots or when a claim matters:

- Capture the starting image.
- Capture the ending/current image.
- Report exact live map and coordinates from `status`.
- If the run failed, say what failed and preserve evidence instead of smoothing it over.

The useful standard is not "looked plausible"; it is live state, input result, and screenshot evidence.
