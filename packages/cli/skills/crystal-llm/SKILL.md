---
name: crystal-llm
description: Use the local crystal-llm Pokemon Crystal MCP over streamable HTTP on this machine. Use when Codex needs to play, inspect, debug, or automate Pokemon Crystal through the persistent localhost MCP session, capture screenshots, update route-learning memory, or diagnose direct-HTTP harness issues.
---

# Crystal LLM

Use the persistent local MCP session for Pokemon Crystal.

- Endpoint: `http://127.0.0.1:3003/api/mcp?session_id=codex-service`
- Continuity save: `$POKECRYSTAL_REPO/apps/web/mcp-codex-service-autosave.sav`
- Server: Docker `pokecrystal-ts` service exposing `@pokecrystal/web` on host port `3003`
- Shortcut: `poke` is installed at `$HOME/.local/bin/poke` and wraps `$CODEX_HOME/skills/crystal-llm/scripts/poke.mjs`
- Bundled command script: this skill includes `scripts/poke.mjs` for installs, repo packaging, and downloadable zips.
- Codex memory: `$CODEX_HOME/pokecrystal/`
- Learning logs: `$CODEX_HOME/pokecrystal/learnings/`

## Required Start

1. The Docker MCP server is expected to already be running on `127.0.0.1:3003`. Do not start, restart, rebuild, or inspect Docker during scheduled gameplay. If `poke status` cannot reach the game, fail fast and report the harness outage.

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
- Read [references/ryan-blog.md](references/ryan-blog.md) before any scheduled play/post cycle for Ryan.

## Live Play Loop

Use this loop for manual and autonomous play:

1. Call `poke status`.
2. If in dialogue, menus, battle, or a naming screen, read and preserve the meaning of the text before clearing it. Dialogue is gameplay evidence, not noise.
3. Call `poke observe` before terrain decisions.
4. Move one tile at a time near trees, ledges, fences, grass pockets, NPCs, or route branches.
5. If the same move fails twice, treat it as a model/tactic failure; observe, identify why the tactic was bad, and choose a different lane.
6. Use `recent_events` after surprising movement, map transitions, story dialogue, NPC movement, or tool errors.
7. Write the lesson if a route assumption, UI trap, or harness failure mattered.

Do not use repo files, ROM data, save internals, emulator memory, implementation coordinates, or generated map data to decide gameplay. For gameplay, use only live MCP outputs, real screenshots, visible text, recent in-game events, controller inputs, and the user's directions. File access is allowed for skill/runner/memory maintenance, not for in-game navigation.

## Menu Navigation Discipline

Menus, prompts, dialogue boxes, naming screens, and battle command panes are live state machines. Never navigate them from memory or from an earlier attempt. Before choosing inputs, identify the current state from the observed surface, selected row, prompt/dialogue flags, visible text, and available controls.

- Treat every modal boundary as a checkpoint. A press that clears text, opens a prompt, closes a prompt, returns to a parent menu, enters a submenu, or exits to overworld must be followed by an observation or compact TUI state check before the next navigation decision.
- Do not combine state-changing acknowledgement inputs with navigation inputs in one shell command. Dialogue and prompt advancement can land on different surfaces depending on script state.
- Use short macros only when the start state and target state are both explicit. A macro should cross one predictable boundary, such as moving from an observed menu row to another visible row, confirming the highlighted option, or backing out one menu layer.
- Derive cursor movement from the currently highlighted row, not from a memorized absolute position. If the selected row is unexpected, recalculate from what is visible.
- When an action fails or opens an error message, clear the message, observe the returned state, and recover from that state. Do not replay the original input sequence.
- For any multi-step item, ability, key item, field move, equipment, or party action: verify each intermediate screen, choose only from observed valid options, and return to the original blocker/objective to prove the action had the intended effect.

## Story Objective Completion

Track objectives symbolically, not just by movement or partial fights. For every story subgoal, keep a small completion ledger in the run summary or durable notes:

- `objective`: the actual story task in generic terms, such as clearing the current dungeon, unlocking the next gate, or opening the next major route.
- `expectedCapstone`: the kind of confirmation that should happen when it is done, such as an NPC arriving, a rescued character, a reward, a forced scene, a guard moving, a warp opening, or the next blocker disappearing.
- `currentEvidence`: dialogue, map transitions, defeated trainers, item/reward text, NPC position changes, and route blockers tested live.
- `stillBlockedBy`: any visible NPC, trainer, locked door, cut tree, or unexplored branch that still prevents the next story objective.
- `nextProofStep`: the next concrete live check that would prove completion or reveal the remaining task.

Do not mark a story objective complete just because the character entered the area, beat one trainer, reached an early room, or returned to town. A story objective is complete only after a symbolic capstone or state change is verified. Examples of valid completion evidence include a relevant NPC arriving or talking after progress, hostile blockers leaving, a guard no longer blocking a gym or route, a reward/key item text, a badge/TM text, or a new warp/door becoming usable.

When an NPC or the story gives a hint, treat it as the primary route clue. If a relevant NPC appears, moves, gives a speech, or reacts to progress, stop and update the ledger before pathing. Do not downgrade explicit instructional dialogue into flavor text. If text says to use, teach, equip, show, deliver, return, buy, heal, or bring something, the next proof step must be that concrete action or a live verification that the action is impossible right now.

When a reward, hint, item, or obstacle points to an overworld HM/TM field move, be precise: use observed TUI/map text to identify what is actually cuttable, surfable, pushable, etc.; distinguish the obstacle type, teach/use the correct move from live menus, then retest the original blocker before rerouting.

If a gym/route remains blocked after a partial dungeon clear, infer that the dungeon/story task is not done and return to find the unvisited branch, remaining trainer, boss, item, or capstone event. If a route remains blocked immediately after receiving an explicit tool/hint for that exact blocker, infer that the missing step is applying the tool/hint, not more random pathing.

## Resource And Catching Policy

- Treat catching Pokemon as useful preparation for beating the game, not as a side objective that overrides story progress.
- Catch 'em all posture: when resources and safety allow, try to catch new species, reasonable dupes with useful roles, and Pokemon that improve party depth, type coverage, HM utility, or backup strength.
- When visiting towns, use marts more proactively. Buy more Potions and Poke Balls or better balls than the old conservative policy would, while preserving enough money for emergency survival.
- Prefer leaving town with enough Balls to make multiple capture attempts and enough Potions to extend routes without constant Pokemon Center retreats.
- When the game offers a nickname prompt after receiving or catching a Pokemon, prefer giving that Pokemon a short nickname unless doing so would block urgent progress.

## Battle And Training Policy

- Training means committing to battles, not dodging them. Fight ordinary wild Pokemon and reachable trainers instead of avoiding encounters or escaping for comfort.
- Do not run from training battles just because HP is low, the matchup is bad, or fainting is likely. Keep making concrete battle actions, switches, item uses, and attacks until the battle is won, the party wipes, or a hard tool/UI failure prevents further input.
- Low HP is not a reason to retreat to town, leave grass, or abandon active training. Treat fainting and whiteout as acceptable training costs, then continue from the resulting live state.
- Treat money as a tracked training resource. Check `poke status` before and after trainer battles, whiteouts, marts, and Mom-bank interactions; record wallet money and Mom's money when either changes or remains suspiciously stuck at zero.
- When the run is corrected for rushing, being too objective-focused, or needing training, treat that as an explicit override: stop the badge/story push, leave the gym or capstone route if needed, and make trainer hunting, wild EXP, catches, supplies, and party development the active goal until live evidence shows the team improved.
- Training is not only incidental travel work. Deliberately explore nearby routes, side buildings, grass, and reachable NPC/trainer lanes to find missed trainer fights and safe EXP before retrying a wall such as a gym leader.
- Training means earning EXP and levels across the team, not just powering one lead. Default to real battles that produce at least one verified level gain for an underleveled or neglected party member when feasible.
- When a boss, gym leader, route, or user correction says the team is not viable, make balanced team readiness the active goal. Set an approximate target level range from the live obstacle and local memory, then keep training until multiple usable non-starter teammates are near that range, not merely until the starter can carry harder.
- For training blocks, inspect the live party before and after; verify changed levels, HP, status, and any move-learning text from live UI.
- Use wild grass deliberately for experience when the team is behind the area, the next trainer/gym looks risky, or recent fights were close. Short training loops are useful progress, not a stall, when they produce EXP.
- Train the whole usable party. Do not funnel routine EXP into only one Pokemon or mostly into the starter. Rotate the lead, use switch training, and give weaker, new, HM-utility, or type-coverage teammates safe KOs until the party is reasonably even for the area.
- Treat non-starter development as the priority during training blocks. The starter may finish dangerous fights, but routine wild encounters, rematch preparation, and safe trainer KOs should build the rest of the team first.
- Before retrying a known wall after a training override, verify party progress from live UI or battle evidence: levels gained, stronger non-starter moves, healthier team distribution, or several non-starter KOs. Do not return just because the starter gained another level.
- Use the strongest Pokemon as the safety valve for dangerous fights, not as the default recipient for every ordinary trainer or wild encounter.
- Heal less often. Do not walk back to a Pokemon Center merely because HP is not full. Keep moving or training while the active Pokemon has comfortable HP, usable PP, and no dangerous status problem.
- Use items, switches, or party depth before retreating all the way to a Center when that keeps the run moving safely.
- Retreat or use a Pokemon Center when there is a concrete survival reason: red HP, repeated near-KOs, badly depleted PP, poison/burn/sleep creating travel risk, multiple fainted Pokemon, or an important upcoming fight.
- Avoid reckless loss loops. Do not chain red-HP encounters unless the user has explicitly allowed whiteout training or there is a deliberate reset/whiteout plan.

## Navigation Rules

- Treat the user's directions as route intent, not blind button macros.
- Follow visible lanes and shelves. Destination direction may require moving away from the destination first.
- Before starting a navigation run, audit the last 5-10 movement events and summarize the path shape: net direction, repeated blocked tiles, branches just proven dead, and the last useful progress landmark. If recent movement drifts away from the stated target or re-enters a known dead pocket, stop and re-anchor on the target landmark from `route --no-image` before issuing another move.
- Pathfind from the live map topology, not from remembered shapes. Use `observe --grid` and `route --no-image` to identify the actual connected floor, chokepoints, branch ends, trainer/NPC gates, item pockets, and warps on the current map. A previous map shape is not a template for the next map.
- When a target is visible in `route --no-image`, pathfind toward it as a graph problem. Treat passable floor, valid ledges, doors, approach tiles, and opened cut-tree spaces as nodes; treat water, walls, trees, invalid ledges, and blocked NPC tiles as blocked. Choose a next waypoint that measurably reduces route distance through connected floor, not just screen distance. If a chosen segment fails twice, mark only that edge blocked and re-plan from the current live node.
- Around water, trees, ledges, and other dense blockers, trace the connected floor rather than the visual direction alone: follow open `.` lanes around the obstacle, check the next opening in TUI, then confirm the broader bend in the tile render. Do not call a pocket a dead end while either view still shows adjacent outgoing floor.
- Do not confuse a blocked adjacent tile with a blocked route. If the next tile in the desired direction is water, tree, wall, cliff, or ledge, inspect the perpendicular open floor and keep following the connected lane around the obstacle.
- Explore by checkpointed frontier, not by wandering. Maintain a small frontier list for the current map/region: `confirmed dead`, `checked but useful`, `unvisited lead`, and `target landmark`. After testing a branch, record the result before leaving it; after moving 4-6 tiles without reaching a landmark, observe and verify the new position still reduces route uncertainty toward the target.
- On dungeon/story maps, push the reachable topology hard: inspect each branch, challenge trainers/NPCs that occupy the route, collect reachable progress items when safe, and continue until the objective ledger has a capstone or a concrete survival reason to leave.
- Warps are leads, not completion. Entering or finding a warp proves only that a branch exists; story completion still requires a capstone, blocker change, reward/key text, boss/trainer clear, or next-route access verified live.
- Treat `d`, `l`, and `r` ledge glyphs as one-way terrain. Cross them only deliberately and only from the valid side.
- Prefer reversible exploration. Keep a route back to the current lane.
- On multi-floor or multi-warp maps, maintain a navigation ledger before moving: current map/coords, current objective, intended next warp/landmark, last useful warp used, local failed moves, and a recovery move if the next lane fails.
- Treat `route --no-image` as the floor-level warp graph when the viewport is too local. Identify the next target warp or landmark from the live render, then move through visible lanes toward it in small verified chunks.
- A blocked tile only invalidates that immediate tile, not the floor objective. After two blocked moves in a pocket, observe, call `route --no-image`, return to the last useful warp or choose a different visible lane, and continue unless HP, battle, or UI state makes movement unsafe.
- When an NPC or trainer blocks a direct line to a target warp, interact or fight if appropriate; if dialogue only or still blocked, route around using adjacent visible lanes instead of ending the run.
- When a major route or gym remains blocked, do not keep testing that blocker. Use it as a symbolic failure signal: identify the unfinished prerequisite, update `stillBlockedBy`, and return to the relevant story area until the capstone state change occurs.
- Do not claim a route is impossible from a single viewport; say "not found in the observed viewport" unless a larger scouted chart proves it.
- Capture start/end proof images when the user asks for screenshots or when a run is being used as evidence.

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

Use TUI and tile views together: TUI/`observe --grid` is best for exact text, selected menu rows, prompts, and local obstacle tokens; `route --tiles` or `route --no-image` is best for spatial layout, connected floors, warps, ledges, water, and tree clusters. For HM/TM field moves or blockers, cross-check both before deciding what is actually usable.

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
      "regionMemory": {
        "<regionName>": {
          "bounds": { "x": [0, 999], "y": [0, 999] },
          "landmarks": ["Target warp, NPC, sign, item, cut tree, ledge, or trainer"],
          "workingPaths": [
            {
              "from": "Observed landmark or coordinate band",
              "to": "Target landmark",
              "waypoints": ["Landmark/coordinate band sequence that worked live"],
              "proof": "Live transition, item disappearance, dialogue, trainer clear, or screenshot/event evidence"
            }
          ],
          "deadFrontiers": [
            {
              "frontier": "Branch or edge that was tested",
              "reason": "Terrain, one-way ledge, headbutt tree, NPC, water, etc.",
              "testedFrom": "Approach tile or coordinate band"
            }
          ],
          "unvisitedLeads": ["Remaining visible branches or approach tiles"],
          "preferredNextWaypoint": "Next landmark to pathfind toward from this region"
        }
      },
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

Store durable navigation knowledge as landmarks, regions, and tested edges, not as blind button strings. A good note says "from shrine pocket, the west wall lane reaches the sign-side upper pocket but the east branch at 17,29 dead-ends at headbutt-tree terrain." A bad note says "go left, up, up, right" without the landmarks and blocker proof.

## Autonomous Runs

Before moving, write a short run plan in durable notes or the runner summary:

- live map and current goal
- current route memory for that map
- recent movement audit: last 5-10 movement events, net displacement, repeated blockers, dead pockets just proven, and the last useful landmark
- recent failed attempts on that map
- story completion ledger: objective, expected capstone, current evidence, remaining blocker, and next proof step
- current navigation ledger for multi-warp or maze maps: current floor, target floor/warp/landmark, last useful warp or landmark, checked/dead frontiers, failed local moves, and recovery lane
- next 2-3 tactics
- survival/training policy for the active Pokemon, including whether to seek wild EXP before the next story fight

Scheduled play should do enough real interactions to matter, usually around 30. Failed movement is a model/tactic problem, not a "blocking" excuse. It should not quit after one failed movement; it should branch, interact, inspect, and keep trying unless HP, battle state, or UI state makes further play unsafe. For story tasks, "enough interactions" means pursuing the capstone state change, not sampling the first quarter of the area and leaving.

For Ryan scheduled play/post cycles, posting is mandatory for every completed run. Always write a local run summary, retain a public trainer journal under `$CODEX_HOME/pokecrystal/blog-posts/`, update `index.json`, and call `$CODEX_HOME/pokecrystal/bin/agent-progress.cjs post-text` with the retained public content. If gameplay produced no milestone, battle, catch, level, item, heal, route unlock, or story capstone, still post a diegetic trainer journal about the observed stall, detour, training attempt, blocked lane, recovery, or survival choice. Do not use no-progress as a reason to skip, suppress, or only save private facts. If the API call fails, preserve the retained public journal and sanitized retry metadata as pending instead of dropping it.

Codex gameplay automation must use the stock `poke` shortcut first (`status`, `observe`, `route`, `context`, `proof`, `move`, `press`, `clear`, `events`) and only use raw MCP calls for tools that wrapper does not expose. Do not route gameplay through `mcporter`.

Do not convert failed behavior into future guidance. Never write direction-order churn, scout-looping, or repeated same-tile pushing as a best practice. Only infer a stall lesson after an actual movement action fails. Diagnostic-only status/observe/context/proof runs must not create stall guidance.

## Action Posture

Use a brash, aggressive dungeon-crawl posture in play decisions: move with purpose, challenge blockers, finish rooms, and prove the task through live state changes. Keep the same player identity and name; only the tactical tone changes.

Do not be passive around trainers. If a trainer is reachable and belongs to the current objective path, challenge them unless survival state says doing so is reckless. Treat trainer clears as both progress and experience, not optional scenery.

Battle inputs must be deliberate. Read the battle state, choose a useful move, switch, item, or ball on purpose, and avoid blind confirm-button loops. Default toward strong STAB or type-advantaged damage, use captures when a useful or new wild Pokemon is practical, and spend healing items before abandoning a dungeon when that keeps the push alive.

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
