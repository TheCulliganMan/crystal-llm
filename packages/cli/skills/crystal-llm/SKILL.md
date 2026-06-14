---
name: crystal-llm
description: Use the local crystal-llm Pokemon Crystal MCP over streamable HTTP on this machine. Use when Codex needs to play, inspect, debug, or automate Pokemon Crystal through the persistent localhost MCP session, capture screenshots, update route-learning memory, or diagnose direct-HTTP harness issues.
---

# Crystal LLM

Use the persistent local MCP session for Pokemon Crystal. This skill is for live gameplay, route recovery, memory maintenance, screenshot/proof capture, and direct-HTTP harness diagnostics.

## Runtime Facts

- MCP endpoint: `http://127.0.0.1:3003/api/mcp?session_id=codex-service`
- Continuity save: `$POKECRYSTAL_REPO/apps/web/mcp-codex-service-autosave.sav`
- Server: Docker `pokecrystal-ts` service exposing `@pokecrystal/web` on host port `3003`
- Preferred shortcut: `poke`, installed at `$HOME/.local/bin/poke`
- Shortcut target: `$CODEX_HOME/skills/crystal-llm/scripts/poke.mjs`
- Bundled script: `scripts/poke.mjs`
- Codex memory root: `$CODEX_HOME/pokecrystal/`
- Learning logs: `$CODEX_HOME/pokecrystal/learnings/`

## Decision Priority

When instructions or evidence conflict, follow this order:

1. The user’s current explicit instruction.
2. Harness safety and mode rules.
3. Live game evidence from MCP outputs, screenshots, visible text, recent events, and controller results.
4. The current hard story gate and active prerequisite.
5. Durable memory that is not contradicted by later live evidence.
6. General play policy: aggressive progress, training, catching, supplies, and survival.
7. Ryan scheduled-run posting policy.

Live evidence beats old memory. Later completion proof beats older handoff directives. A progress tracker or status label is a candidate objective, not an order.

## Mode Boundaries

### Scheduled Gameplay

The Docker MCP server is expected to already be running on `127.0.0.1:3003`.

During scheduled gameplay:

- Do not start Docker.
- Do not restart Docker.
- Do not rebuild Docker.
- Do not inspect Docker.
- If `poke status` cannot reach the game, fail fast and report the harness outage.
- Do not use `mcporter` for gameplay.
- Use the stock `poke` wrappers first.

### Harness or Skill Diagnostics

Use harness diagnostics only when the task is explicitly about the harness, the skill, direct-HTTP calls, validation, screenshots, or service failure triage.

Use `mcporter` only when a Codex-local config is available and the task is diagnostic. If `mcporter` says `Unknown MCP server 'pokecrystal'`, use the direct HTTP scripts. Do not infer that the game or MCP server is down from that error alone.

The service restart rule in the Validation section applies only to skill or harness changes, not scheduled gameplay.

## Reference Files

Read these files when relevant:

- `references/mcp-harness.md` for service management, direct HTTP calls, screenshot capture, `mcporter` config, and harness failure triage.
- `references/play-policy.md` before substantial gameplay, route recovery, training, or memory updates.
- `references/ryan-blog.md` before any scheduled play/post cycle for Ryan.

## Allowed Gameplay Evidence

For gameplay decisions, use only:

- `poke status`
- `poke observe`
- `poke observe --grid`
- `poke route`
- `poke route --tiles`
- `poke events`
- `poke context`
- Real screenshots or saved proof images
- Visible in-game text
- Recent in-game events
- Controller inputs and their observed results
- The user’s directions

Do not use repository files, ROM data, save internals, emulator memory, implementation coordinates, generated map data, or external spoilers to decide gameplay.

File access is allowed for skill maintenance, runner maintenance, memory maintenance, and harness diagnostics. It is not allowed as an in-game navigation oracle.

## Stock Tools

Prefer the `poke` shortcut. If `poke` is not on `PATH`, use:

```bash
$CODEX_HOME/skills/crystal-llm/scripts/poke.mjs
```

Preferred compact commands:

```bash
poke status
poke observe
poke observe --image
poke observe --grid
poke route
poke route --image
poke route --tiles
poke proof <label>
poke move left
poke press A
poke clear
poke events
poke context
```

Use `scripts/mcp_call.mjs` only when a raw MCP tool call is needed.

Direct gameplay automation uses the Crystal LLM direct-HTTP scripts, not `mcporter`.

### Tool Selection

Use `poke status` for high-level state and the current flow objective.

Use `poke observe` for:

- TUI text
- Menu rows
- Cursors
- Party order
- Battle ally/enemy state
- HP
- EXP
- Prompts
- Dialogue
- Selected command panes

Use `poke observe --grid` when local terrain tokens matter.

Use `poke route` for:

- Current map layout
- Connected floor
- Warps
- Ledges
- Water
- Tree clusters
- Route branches
- Larger spatial context

Use `poke route --tiles` when tile-level interpretation matters.

Use image capture only when needed:

```bash
poke observe --image
poke observe --save-images <dir>
poke route --image
poke proof <label>
```

Request images when:

- The user asks for visual proof.
- TUI or route text is insufficient.
- Navigation is going poorly.
- Repeated movement fails.
- A target remains missed after several moves.
- The user corrects the route shape.
- The agent is making bad spatial assumptions.
- A run needs evidence.

Use `poke context` instead of reading the full learning-state JSON when only the current map, next prompt, and route memory are needed.

## Core Live Play Loop

Use this loop for manual and autonomous gameplay.

1. Call `poke status`.
2. Reconcile the live next objective against route memory, recent events, the active trajectory, and the last observed blocker.
3. If the status or flow objective is gated by a missing item, badge permission, party capability, story flag, NPC reward, dialogue handoff, or unresolved prerequisite, make the prerequisite the immediate objective.
4. Call `poke observe` before terrain, menu, battle, and dialogue decisions.
5. If in dialogue, menu, battle, prompt, or naming screen, read the surface before pressing inputs. Preserve the meaning of important text before clearing it.
6. Move one tile at a time near trees, ledges, fences, grass pockets, NPCs, route branches, water, holes, stairs, and warps.
7. If the same move fails twice, treat the tactic as wrong. Observe, identify why it failed, and choose a different lane or interaction.
8. Use `poke events` after surprising movement, map transitions, story dialogue, NPC movement, tool errors, or unexpected UI state.
9. Write a lesson when a route assumption, UI trap, memory problem, or harness failure affected the run.

Dialogue is gameplay evidence. Do not treat it as noise.

## Objective Reconciliation

At the start of any route recovery, dungeon, gym maze, repeated failure, or scheduled run, reconcile the objective before moving.

Record or reason through:

- `flowObjective`: what `poke status` or the tracker says.
- `hardGate`: the broader story gate.
- `activePrerequisite`: the nearest unfinished prerequisite that can change the gate.
- `memoryBlocker`: what durable memory says is blocked or impossible.
- `liveEvidence`: the most recent observed proof.
- `lastBlockingProof`: the refusal, blocked route, missing item, missing permission, or missing capstone.
- `nextProofStep`: the next concrete live check or action that can prove progress.
- `completionProofNeeded`: the evidence that will retire the prerequisite.

Do not confuse a gate label with the next action. If a clerk, guard, route, counter, item, boss, NPC, or obstacle refuses because a prerequisite is missing, record the refusal and pursue the source of the missing prerequisite. Repeating the refusal is not progress until live evidence changes.

## Local-Minimum Guard

Never let a proven checkpoint become the objective.

A completed proof is not progress when repeated. Do not re-verify a completed prerequisite, re-enter a solved room, retalk exhausted NPCs, or publish another post about the same proof unless the user explicitly asks or fresh live evidence contradicts completion.

Treat any of these patterns as a local-minimum failure:

- Returning to the same solved room.
- Rechecking the same NPC.
- Testing the same blocked route edge.
- Repeating the same shop/counter/refusal.
- Reposting the same public milestone theme.
- Looping around the same route pocket.
- Trusting a broad status label after live evidence proved the missing prerequisite.

When stuck in a repeated loop:

1. Retire the memory or prompt wording that caused it.
2. Preserve one compact durable proof fact.
3. Set the immediate objective to the larger unfinished story task or its active prerequisite.
4. Route away from the stale attractor.
5. Do not turn the repeated proof into another public milestone.

Public journals must not reward local minima. A repeated verification, blocked edge, or room return should be summarized as stale and retired in private notes, not celebrated as progress.

## Long-Horizon Objective Guard

Maintain a compact trajectory ledger for the current hard story gate.

The trajectory ledger outranks short-term route labels when live evidence proves that the label is only a blocked gate name. A progress tracker may name the broad gate, but the immediate action must be the nearest unfinished prerequisite that can unlock that gate.

Before scheduled play, reconcile:

- Hard gate
- Active prerequisite
- Last proof that the prerequisite is incomplete
- Stale local attractors
- Route-away waypoint
- Next proof step
- Completion proof

A useful run must either:

- Advance the active prerequisite.
- Gather new evidence about that prerequisite.
- Move toward the next proof step.
- Produce training, supplies, catches, healing, or route knowledge that directly supports the active prerequisite.

Rechecking a known refusal or re-entering a solved building is not useful progress.

When a gate is blocked by missing story authorization, request text, NPC handoff, or capstone dialogue, pursue the story source that creates that authorization. Do not keep testing the blocked service, clerk, route, or obstacle until that story source is verified.

Use these durable fields where applicable:

```json
{
  "trajectoryLedger": {
    "hardGate": "",
    "activePrerequisite": "",
    "staleAttractors": [],
    "lastBlockingProof": "",
    "nextProofStep": "",
    "completionProof": "",
    "completionProofNeeded": ""
  }
}
```

If a stale attractor is retired, remove or compact older notes whose active objective still points at it.

## Story Objective Completion

Track story objectives symbolically, not merely by movement, partial fights, or reaching an area.

For every story subgoal, maintain a compact completion ledger in the run summary or durable notes:

```json
{
  "objective": "",
  "expectedCapstone": "",
  "currentEvidence": "",
  "stillBlockedBy": "",
  "nextProofStep": "",
  "objectiveReconciliation": {
    "flowObjective": "",
    "memoryBlocker": "",
    "requiredPrerequisite": "",
    "liveEvidence": "",
    "nextProofStep": ""
  },
  "npcLedger": [],
  "buildingLedger": [],
  "subtaskLedger": {
    "hardGate": "",
    "immediateSubtask": "",
    "whyItAdvancesGate": "",
    "proofNeeded": "",
    "checkedLeads": [],
    "nextUncheckedLead": ""
  }
}
```

A story objective is not complete merely because the player:

- Entered the area.
- Reached an early room.
- Beat one trainer.
- Returned to town.
- Found a warp.
- Saw the broad location named by the tracker.

A story objective is complete only when a symbolic capstone or state change is verified live. Valid completion evidence includes:

- Relevant NPC arrival or changed dialogue.
- Rescued character.
- Reward text.
- Key item text.
- Badge or TM text.
- Hostile blocker leaving.
- Guard no longer blocking a gym or route.
- Warp, door, or route becoming usable.
- Boss or required trainer clear.
- Explicit story handoff.

When NPC dialogue gives a hint, treat it as a primary route clue. If text says to use, teach, equip, show, deliver, return, buy, heal, or bring something, the next proof step must be that concrete action or a live verification that the action is impossible right now.

If a gym, route, service, or story gate remains blocked after a partial dungeon clear, infer that the story task is not done. Return to find the unvisited branch, remaining trainer, boss, item, NPC, or capstone event.

For towers, dungeons, and multi-floor story buildings, reaching the building or an early floor is not enough. Continue through stairs, holes, side rooms, trainer lanes, warps, and reachable branches until the top-floor or capstone evidence is observed, or until live evidence proves a different prerequisite.

## Subtask Discipline

When the hard gate is broad, do not make the gate label the action. Choose a concrete subtask that can produce live evidence.

Use a compact `subtaskLedger`:

```json
{
  "hardGate": "",
  "immediateSubtask": "",
  "whyItAdvancesGate": "",
  "proofNeeded": "",
  "checkedLeads": [],
  "nextUncheckedLead": ""
}
```

Update or replace the subtask after each:

- Capstone
- Refusal
- Dead branch
- NPC clue
- Reward
- Warp transition
- Trainer clear
- Route failure
- Memory contradiction
- User correction

## Fact-Finding and NPC Clues

Use this protocol when the task is to find a clue, verify whether an NPC says something, identify where a reward or hint comes from, or debug a claim that seems to improve only after restart.

Treat fact-finding as evidence gathering, not route assumption.

Build a small checked list:

- Candidate map, town, route, house, cafe, gate, cave, shop, or center.
- NPC or overworld person approached.
- Exact dialogue meaning.
- Whether the dialogue proves the fact.
- Reward, item, HM, TM, flag, or story hint if any.
- Next unchecked lead.

Maintain an `npcLedger` for clue, reward, and story searches:

```json
{
  "npcLedger": [
    {
      "map": "",
      "locationOrLandmark": "",
      "approachEvidence": "",
      "dialogueMeaning": "",
      "rewardOrFlag": "",
      "resolved": false,
      "nextFollowUp": ""
    }
  ]
}
```

Maintain a `buildingLedger` for towns, routes with houses, gates, caves, shops, centers, and special venues:

```json
{
  "buildingLedger": [
    {
      "map": "",
      "localLandmark": "",
      "entered": false,
      "notableNPCs": [],
      "cluesOrRewards": [],
      "uncheckedNPCsOrRooms": [],
      "exitOrReturnEvidence": "",
      "exhaustedForCurrentObjective": false
    }
  ]
}
```

When a route objective is blocked by a missing capability, item, story flag, or unknown clue, prioritize unchecked NPCs and buildings in the relevant town, route, or nearby hub before repeating travel to the blocker.

Do not mark a hub exhausted after checking only the obvious outdoor path. Inspect reachable buildings, side rooms, counters, and NPC clusters unless survival state makes that unsafe.

When a lead says “house,” “cafe,” “near town,” “west of town,” or similar, inspect all live nearby warps and outdoor NPCs in that region before concluding the clue is absent.

Talk to candidate NPCs one at a time:

1. Stand in a valid interaction lane.
2. Press `A` once.
3. Observe immediately.
4. Preserve the dialogue meaning.
5. If there are multiple pages, advance one page and observe again.
6. Record the evidence before clearing the final text.

A plausible but different clue is not completion. Record it as checked and continue.

Finish fact-finding with evidence, not confidence: report the exact dialogue or state change that proves the fact, plus nearby candidates that were checked and did not prove it.

## Same-Session Bug Investigation

When the user reports a bug that appears after a while, keep using the same live session long enough to exercise the suspected stale state.

A restart alone is not proof. Use repeated dialogue, map transitions, menus, prompts, and boundaries in the same session.

If the stale-state bug reappears, capture:

- Current surface
- Recent events
- Exact failing input
- Expected behavior
- Actual behavior
- Whether the failure survives another observation or transition

Only then debug or restart.

## Historical Memory Use

Use durable memory as live route evidence, not background flavor.

At the start of any route recovery, dungeon, gym maze, or repeated failure:

1. Call `poke context`.
2. Match the live map and coordinates to `routeMemory[mapName].regionMemory`.
3. Summarize applicable landmarks, working paths, dead frontiers, unvisited leads, current hypothesis, and next prompt.
4. Treat matching `appliesWhen` or `bounds` entries as the current map-local playbook unless live evidence contradicts them.

Combine live objectives with memory. Do not blindly follow either.

The correct immediate objective is the nearest proven prerequisite that advances the larger story objective, such as:

- Current target
- Missing capability
- Remembered NPC or reward
- Known item lead
- Story handoff
- Route blocker solution
- Training or supply need
- Obstacle use

If flow status points through a route that memory says is impossible without a missing capability or story state, stop routing to that blocker and pursue the prerequisite.

Before following urgent memory such as “do not leave,” “return to,” “must obtain,” “unchecked lead,” or “active handoff,” scan nearby memory for later completion proof. Later proof wins. Update stale directives before playing more.

Use `npcLedger` and `buildingLedger` as first-class memory alongside route memory. Before concluding that a prerequisite is unknown or impossible, review unchecked NPCs and buildings in the current hub and remembered clue hubs.

Apply remembered blocker solutions before inventing a detour. If memory says a prompt, item, NPC interaction, warp, or menu action solves the blocker, verify the live blocker and use the solution from the observed UI.

Never re-test a remembered dead frontier unless live evidence shows the map state changed. Mark only the immediate edge or object as dead; do not generalize one blocked tile into an impossible route.

Prefer remembered working paths as landmark sequences, then re-derive the next few inputs from live `observe`, `route`, and `route --tiles`. Do not replay blind button strings.

After a remembered route works, fails, or changes, update `routeMemory` with:

- Current map and coordinates
- Target landmark
- Tested edge
- Result
- Reason
- Live evidence

## Memory Contract

Route-specific corrections must live outside this reusable skill.

Canonical files:

- State JSON: `$CODEX_HOME/pokecrystal/poke_learning_state.json`
- Human route notes: `$CODEX_HOME/pokecrystal/poke_learning_journal.md`
- Daily sidecar: `$CODEX_HOME/pokecrystal/memory/YYYY-MM-DD-pokecrystal-learning.md`
- Durable learning log: `$CODEX_HOME/pokecrystal/learnings/LEARNINGS.md`
- Tool/runtime failures: `$CODEX_HOME/pokecrystal/learnings/ERRORS.md`

### Memory Enhancement Procedure

When a correction, repeated-run failure, stale objective, route loop, UI trap, or trajectory mistake is found, improve durable memory before more gameplay. Do not merely add prose warnings.

1. Read the canonical state.
2. Identify the stale attractor: exact NPC, building, route edge, menu, post theme, tracker label, or short-term objective that caused the loop.
3. If story-level, update `trajectoryLedger`.
4. Update `objectiveReconciliation`.
5. Update `nextPrompt` so `poke context` exposes the corrected immediate objective.
6. Prune or compact stale active fields that would pull the next run backward.
7. Preserve one compact proof fact for the retired attractor.
8. Point the relevant preferred next waypoint to the active prerequisite.
9. Add an `objectiveLock` only when stale status labels must not displace the corrected prerequisite.
10. Append journal or daily-memory notes only after the JSON state is corrected.
11. Validate the edit.

Fields to check and clean when retiring stale memory:

- `routeHints`
- `trajectoryLedger`
- `objectiveReconciliation`
- `npcLedger.currentObjective`
- `npcLedger.uncheckedLeads`
- `buildingLedger.currentObjective`
- `buildingLedger.uncheckedLeads`
- `buildingLedger.activeHandoff`
- `buildingLedger.roomLock`
- `subtaskLedger`
- `routeMemory[mapName].regionMemory`
- `preferredNextWaypoint`
- `unvisitedLeads`

Avoid active “do not repeat” warnings unless paired with a concrete next proof step. Stale warnings can become accidental objectives.

Use `objectiveLock` only when needed:

```json
{
  "objectiveLock": {
    "lockedPrerequisite": "",
    "acceptableCompletionProof": "",
    "disallowedRewriteTriggers": [],
    "nextProofStep": ""
  }
}
```

Do not rewrite the locked prerequisite until the acceptable completion proof is observed live.

### Memory Validation

After editing durable memory:

```bash
python3 -m json.tool $CODEX_HOME/pokecrystal/poke_learning_state.json >/dev/null
python3 $CODEX_HOME/skills/.system/skill-creator/scripts/quick_validate.py $CODEX_HOME/skills/crystal-llm
poke context
```

If the MCP server is timing out, record the validation gap.

### Route Memory Schema

Store route-specific data under `routeMemory[mapName]`:

```json
{
  "routeMemory": {
    "<MapName>": {
      "privateNavigationHint": "Private human correction or learned route note.",
      "regionMemory": {
        "<regionName>": {
          "bounds": {
            "x": [0, 999],
            "y": [0, 999]
          },
          "landmarks": [
            "Target warp, NPC, sign, item, cut tree, ledge, or trainer"
          ],
          "workingPaths": [
            {
              "from": "Observed landmark or coordinate band",
              "to": "Target landmark",
              "waypoints": [
                "Landmark or coordinate band sequence that worked live"
              ],
              "proof": "Live transition, item disappearance, dialogue, trainer clear, screenshot, or event evidence"
            }
          ],
          "deadFrontiers": [
            {
              "frontier": "Branch or edge that was tested",
              "reason": "Terrain, one-way ledge, headbutt tree, NPC, water, etc.",
              "testedFrom": "Approach tile or coordinate band"
            }
          ],
          "unvisitedLeads": [
            "Remaining visible branches or approach tiles"
          ],
          "preferredNextWaypoint": "Next landmark to pathfind toward from this region"
        }
      },
      "privateNavigationPlan": {
        "appliesWhen": {
          "x": [0, 999],
          "y": [0, 999]
        },
        "routeSteps": [
          "Observe current lane.",
          "Move one tile.",
          "If it fails twice, branch to a different tactic."
        ],
        "inspectEvery": 5
      }
    }
  }
}
```

Do not store generic direction orders as route guidance. `routeHints` is legacy and should remain empty except during migration of old private notes into `routeMemory`.

Good memory names landmarks, regions, tested edges, and proof.

Bad memory stores blind strings like “left, up, up, right” without landmarks or blocker evidence.

## HM Capability Ledger

Keep a compact `hmCapabilityLedger` only for observed HM ownership, known user, and badge usability.

Check it when:

- The immediate route is blocked by an HM-style obstacle.
- A run just received or taught an HM.
- A live menu or party state affects immediate obstacle use.

Do not run broad clue searches or repeat status checks solely because an HM exists.

When an HM is proven owned or usable:

1. Preserve one capability fact.
2. Remove stale “missing HM” objectives.
3. Remove old handoffs, room locks, NPC searches, building searches, unchecked leads, route hypotheses, and route-memory entries whose purpose was obtaining that HM.
4. Keep only route memory that helps current navigation or obstacle use.

Do not leave repeated “do not reacquire” reminders in active objective fields.

If field use fails despite ownership proof, record the live failure precisely:

- Wrong shoreline tile
- Missing badge permission
- Wrong party member
- Fainted user
- Prompt not facing water
- Move not currently in party
- Wrong obstacle tile
- Incorrect menu state

Treat that as immediate obstacle diagnosis, not a reason to reacquire the HM, unless the live bag or menu proves the HM absent.

## Menu, Prompt, Dialogue, and Battle Discipline

Menus, prompts, dialogue boxes, naming screens, PCs, counters, signs, item balls, shelves, machines, and battle command panes are live state machines.

Never navigate them from memory or from a previous attempt.

Before choosing inputs, identify:

- Current surface
- Selected row
- Prompt or dialogue state
- Visible text
- Available controls
- Current menu layer
- Whether the next press will clear text, confirm, cancel, enter a submenu, or return to the overworld

Treat every modal boundary as a checkpoint. After any input that clears text, opens a prompt, closes a prompt, returns to a parent menu, enters a submenu, exits to overworld, changes party selection, or changes battle command pane, observe before the next navigation decision.

Do not combine state-changing acknowledgement inputs with navigation inputs in one command. Dialogue and prompt advancement can land on different surfaces depending on script state.

Use short macros only when the start state and target state are explicit, and only across one predictable boundary.

Derive cursor movement from the currently highlighted row. If the selected row is unexpected, recalculate.

When an action fails or opens an error message:

1. Preserve the message meaning.
2. Clear the message.
3. Observe the returned state.
4. Recover from that state.
5. Do not replay the original sequence blindly.

For multi-step item, ability, key item, field move, equipment, or party actions:

1. Verify each intermediate screen.
2. Choose only observed valid options.
3. Return to the original blocker or objective.
4. Prove the action had the intended effect.

### Object Interaction

For PCs, counters, signs, item balls, shelves, machines, and similar objects, side-facing often does nothing.

Do not keep pressing `A` from a side tile after a failed interaction.

Instead:

1. Use live `observe`, `route`, `interactionLane`, or `interactionSetup`.
2. Identify the usable approach tile.
3. Stand directly in front of the object.
4. Face the object.
5. Press `A` once.
6. Verify that text, menu, or dialogue opened.

### Battle Party Switching

Use the battle `PKMN` command, then the live party menu.

If selecting a Pokemon opens a submenu, `STATS` is usually the default and `SWITCH` is the next row.

Safe switch procedure:

1. Observe the cursor on the target Pokemon.
2. Press `A` to open the submenu.
3. Observe `SUBMENU`.
4. Press `down` to `SWITCH`.
5. Press `A`.
6. Verify that the active ally changed in TUI before choosing a move.

## Navigation Policy

Treat the user’s directions as route intent, not blind button macros.

Pathfind from live topology. Use visible lanes, shelves, routes, stairs, warps, ledges, holes, grass, water, trees, NPCs, and walls. Destination direction may require moving away from the destination first.

Before a navigation run, audit the last 5-10 movement events:

- Net direction
- Repeated blocked tiles
- Branches just proven dead
- Last useful progress landmark
- Whether movement drifted away from the stated target
- Whether the route re-entered a known dead pocket

If recent movement is drifting, stop and re-anchor on the target landmark from `poke route` before moving again.

### Graph-Based Route Method

When a target is visible in `poke route`, pathfind as a graph problem.

Treat these as nodes or passable terrain:

- Passable floor
- Valid ledges from the correct side
- Doors
- Stairs
- Holes
- Approach tiles
- Open cut-tree spaces
- Grass
- Warps

Treat these as blocked unless live evidence proves otherwise:

- Water without usable Surf state
- Walls
- Trees
- Invalid ledges
- Blocked NPC tiles
- Closed route gates
- Unusable object sides

Choose a next waypoint that reduces route distance through connected floor, not merely screen distance.

If a segment fails twice, mark only that edge blocked and re-plan from the current live node.

### Frontier Method

Explore by checkpointed frontier, not wandering.

Maintain a compact local frontier list:

```json
{
  "target": "",
  "current": "",
  "visitedEdges": [],
  "deadEdges": [],
  "frontier": [],
  "lastLandmark": "",
  "nextSegment": ""
}
```

Move only 3-6 tiles or to the next junction, warp, trainer, item, or landmark. Then observe and update the ledger.

A segment counts as progress only if it:

- Reaches the named landmark.
- Discovers a new branch, warp, object, NPC, or route.
- Marks a specific edge dead from live evidence.
- Changes floors or maps.
- Produces a useful battle, item, dialogue, catch, heal, or route unlock tied to the current objective.

Repeating a coordinate, reversing the last segment, or drifting to an unrelated hotspot is not progress. Stop, call `poke route --tiles`, and choose a different frontier.

### Dense Blockers

Around water, trees, ledges, fences, and dense blockers, trace the connected floor instead of pushing the visual direction.

Do not confuse a blocked adjacent tile with a blocked route. If the desired tile is water, tree, wall, cliff, or ledge, inspect perpendicular open floor and follow the connected lane around the obstacle.

Do not call a pocket a dead end while TUI or route view still shows adjacent outgoing floor.

### Multi-Floor and Multi-Warp Maps

On multi-floor or multi-warp maps, maintain a navigation ledger before moving:

```json
{
  "currentMap": "",
  "coords": "",
  "currentFloor": "",
  "objective": "",
  "targetFloorOrWarp": "",
  "lastUsefulWarpOrLandmark": "",
  "checkedFrontiers": [],
  "deadFrontiers": [],
  "failedLocalMoves": [],
  "recoveryLane": ""
}
```

For floor-change requests, make the floor-changing warp, stairs, ladder, or hole the immediate target. Do not detour to items, trainers, signs, or broad story labels unless the route to the floor change is blocked by that exact object.

If multiple warps are visible, choose the one whose live target changes toward the requested floor. If the warp is not reachable from the current connected component, route to the nearest connector that could reach it and record the current component as unresolved.

Record floor memory concretely:

- Entry coordinate
- Warp used
- Exit coordinate
- Unexplored frontiers left behind
- Dead edges
- Useful landmarks

Warps are leads, not completion. Entering or finding a warp proves only that a branch exists. Story completion still requires a capstone, blocker change, reward, boss clear, or next-route access verified live.

### Terrain Rules

- Treat `d`, `l`, and `r` ledge glyphs as one-way terrain.
- Cross ledges only deliberately and only from the valid side.
- Prefer reversible exploration when possible.
- Keep a route back to the current lane when survival matters.
- Grass is passable route terrain, not a blocker.
- Cross grass when it is the live route to healing, supplies, town, a gate, or the current objective.
- Do not stand outside grass waiting for a heal-first plan that the map does not offer.

### Route Impossibility Claims

Do not claim a route is impossible from a single viewport. Say “not found in the observed viewport” unless a larger scouted chart proves the route blocked.

A blocked tile invalidates only that immediate tile, not the floor objective.

After two blocked moves in a pocket:

1. Observe.
2. Call `poke route` or `poke route --tiles`.
3. Return to the last useful warp or landmark, or choose a different visible lane.
4. Continue unless HP, battle state, or UI state makes movement unsafe.

If an NPC or trainer blocks a direct line to a target warp, interact or fight if appropriate. If dialogue-only or still blocked, route around using adjacent visible lanes.

## Battle, Training, Catching, and Supplies

Use a brash, aggressive dungeon-crawl posture: move with purpose, challenge blockers, finish rooms, and prove tasks through live state changes.

Keep the same player identity and name. Only tactical tone changes.

### Battle Policy

Battle inputs must be deliberate. Read the battle state and choose a useful move, switch, item, ball, or run command on purpose.

Avoid blind confirm-button loops.

Default toward:

- Strong STAB or type-advantaged damage.
- Safe finishing moves.
- Useful status or debuffs when tactically justified.
- Captures when useful or new wild Pokemon are practical.
- Healing items before abandoning a dungeon when they preserve the push.

Do not be passive around trainers. If a trainer is reachable and belongs to the current objective path, challenge them unless survival state says doing so is reckless. Trainer clears are both progress and experience.

### Training Policy

Training means committing to battles, not dodging them.

Fight ordinary wild Pokemon and reachable trainers when the active battler or a reasonable switch can win without burning the run down.

Do not flee just because:

- HP is imperfect.
- The matchup is mildly bad.
- The fight takes several turns.
- The active Pokemon is not ideal.
- The party is not fully healed.

Run from wild battles when live evidence says the fight is a bad trade:

- Likely wipe
- Severe PP or item drain
- Bad type matchup with no good switch
- Multiple fainted teammates
- Fight blocks the current objective
- Dangerous status or HP pattern
- Capture attempt is no longer worth the cost

If Run fails, choose the next safe action from live state: run again, switch, item, attack, ball, or accept whiteout if that is the best recovery path.

Low HP is not automatically a reason to retreat to town, leave grass, or abandon training. Treat fainting and whiteout as acceptable training costs when the route or objective justifies it. Avoid reckless loss loops.

Do not chain red-HP encounters unless the user explicitly allowed whiteout training or there is a deliberate reset plan.

### Training Overrides

When the user corrects the run for rushing, being too objective-focused, or needing training, treat that as an explicit override.

Stop the badge or story push if needed. Make trainer hunting, wild EXP, catches, supplies, and party development the active goal until live evidence shows the team improved.

Training is not only incidental travel. Deliberately explore nearby routes, side buildings, grass, and reachable trainer lanes to find missed trainer fights and safe EXP before retrying a wall.

Balanced team readiness matters. If a boss, gym leader, route, or user correction says the team is not viable, set an approximate target level range from live obstacle evidence and local memory. Keep training until multiple usable non-starter teammates are closer to that range, not merely until the starter can carry harder.

Before retrying a known wall after a training override, verify party progress from live UI or battle evidence:

- Levels gained
- Stronger non-starter moves
- Healthier team distribution
- Several non-starter KOs
- Improved matchup options
- Better item or PP state

Do not return merely because the starter gained another level.

### Party Training

Training means earning EXP and levels across the usable party.

Prefer:

- Direct KOs when safe.
- Reasonable lead rotation.
- Efficient story-relevant battles.
- Non-starter development during training blocks.

Use the strongest Pokemon as the safety valve for dangerous fights, not as the default recipient for every ordinary trainer or wild encounter.

Switch training is optional and situational. Use it only when training is the active goal and the setup will not waste the run.

Safe switch-training procedure:

1. Put the weak Pokemon first before entering grass or a trainer line.
2. Verify the battle opens with that Pokemon as `ALLY`.
3. Switch immediately to the safety Pokemon before selecting an attack.
4. Win the battle.
5. Verify the weak Pokemon’s EXP or level from live TUI stats.

Do not count switch training as successful if EXP did not change.

Do not waste tempo switching out a 1 HP or near-faint active Pokemon when the opponent is likely to move first or KO it anyway. Let the active Pokemon attack if it can meaningfully damage, debuff, or finish the foe. Otherwise let it faint and choose the replacement from the free switch prompt.

### Healing and Retreat

Heal less often. Do not walk back to a Pokemon Center merely because HP is not full.

Keep moving or training while the active Pokemon has:

- Comfortable HP
- Usable PP
- No dangerous status problem
- Reasonable matchup options
- Party depth

Use items, switches, and party depth before retreating all the way to a Center when that keeps the run moving safely.

Retreat or use a Pokemon Center when there is a concrete survival reason:

- Red HP
- Repeated near-KOs
- Badly depleted PP
- Poison, burn, sleep, or paralysis creating travel risk
- Multiple fainted Pokemon
- Important upcoming fight
- No reasonable switch or item plan

### Catching and Resources

Treat catching Pokemon as useful preparation for beating the game, not as a side objective that overrides story progress.

Catch-’em-all posture:

- Catch new species when resources and safety allow.
- Catch reasonable duplicates with useful roles.
- Prioritize Pokemon that improve party depth, type coverage, HM utility, backup strength, or future training options.

When visiting towns, use marts proactively. Buy more Potions and Poke Balls, or better balls when available, than an overly conservative policy would. Preserve enough money for emergency survival.

Prefer leaving town with:

- Enough Balls for multiple capture attempts.
- Enough Potions or equivalent healing to extend routes without constant Center retreats.
- Enough safety margin for the current dungeon, gym, or route.

When the game offers a nickname prompt after receiving or catching a Pokemon, prefer giving that Pokemon a short nickname unless doing so would block urgent progress.

## Autonomous Runs

Before moving in an autonomous run, write a short run plan in durable notes or the runner summary.

Include:

- Live map and current goal.
- Current route memory for that map.
- Active trajectory reconciliation.
- Hard gate.
- Active prerequisite.
- Stale attractors.
- Route-away waypoint.
- Completion proof.
- Why status or flow label is not enough if it conflicts.
- `hmCapabilityLedger` only if relevant to the immediate blocker or newly received move.
- Recent movement audit.
- Recent failed attempts on that map.
- Story completion ledger.
- Current navigation ledger for multi-warp or maze maps.
- Next 2-3 tactics.
- Survival and training policy for the active Pokemon.
- Whether to seek wild EXP before the next story fight.

Scheduled play should perform enough real interactions to matter, usually around 30. Failed movement is a model or tactic problem, not a blocking excuse.

Do not quit after one failed movement. Branch, interact, inspect, fight, heal, route, or capture as appropriate unless HP, battle state, UI state, or harness failure makes further play unsafe.

For story tasks, enough interaction means pursuing the capstone state change, not sampling the first quarter of the area and leaving.

A scheduled run is not locked to one stale item or errand label. If live evidence shows the gate needs training, supplies, healing, a different NPC clue, a route capability, a medicine pickup, or a prerequisite story event, make that concrete prerequisite the active objective and pursue it.

Codex gameplay automation must use the stock `poke` shortcut first:

```bash
poke status
poke observe
poke route
poke context
poke proof
poke move
poke press
poke clear
poke events
```

Use raw MCP calls only for tools the wrapper does not expose. Do not route gameplay through `mcporter`.

Do not convert failed behavior into future guidance. Never write direction-order churn, scout-looping, or repeated same-tile pushing as a best practice. Only infer a stall lesson after an actual movement action fails. Diagnostic-only status, observe, context, proof, or route runs must not create stall guidance.

## Ryan Scheduled Play and Public Posting

For Ryan scheduled play/post cycles, posting is mandatory for every completed run.

Every completed scheduled run must:

1. Write a local run summary.
2. Retain a public trainer journal under:

```bash
$CODEX_HOME/pokecrystal/blog-posts/
```

3. Update:

```bash
$CODEX_HOME/pokecrystal/blog-posts/index.json
```

4. Call:

```bash
$CODEX_HOME/pokecrystal/bin/agent-progress.cjs post-text
```

using the retained public content.

If gameplay produced no milestone, battle, catch, level, item, heal, route unlock, or story capstone, still post a diegetic trainer journal about the observed stall, detour, training attempt, blocked lane, recovery, or survival choice.

Do not use no-progress as a reason to skip, suppress, or only save private facts.

If the API call fails:

- Preserve the retained public journal.
- Preserve sanitized retry metadata.
- Mark the post as pending.
- Do not drop the public content.

Public posts must not reward stale loops. If the run repeated a known refusal, blocked edge, solved room, exhausted NPC, or retired attractor, summarize it as stale and pivoted away. Do not present the repeated proof as a new milestone.

## Screenshots and Proof

Capture start and end proof images when:

- The user asks for screenshots.
- The run is being used as evidence.
- Navigation is confusing.
- Repeated movement fails.
- The model made bad spatial assumptions.
- Route text is insufficient.
- A story capstone, blocker change, reward, or route unlock should be preserved.

Use:

```bash
poke proof <label>
poke observe --image
poke observe --save-images <dir>
poke route --image
```

Base subsequent movement on inspected proof when route text alone is ambiguous.

## Failure Handling

### Movement Failure

If a move fails twice:

1. Stop repeating it.
2. Observe.
3. Use `poke route` or `poke route --tiles`.
4. Identify the actual blocker.
5. Mark only that edge or tile as blocked.
6. Choose another lane, interaction, or route target.

Failed movement is a tactic failure unless live evidence proves a real blocker.

### UI Failure

If an input opens the wrong menu, prompt, message, or submenu:

1. Preserve the text or selected state.
2. Clear or back out one state at a time.
3. Observe after each modal boundary.
4. Resume from the observed state.
5. Do not replay the failed macro.

### Story Failure

If a major route, gym, clerk, NPC, item, counter, or obstacle remains blocked:

1. Record the refusal as gate evidence.
2. Identify the missing prerequisite.
3. Update `stillBlockedBy`.
4. Route away from the refused target.
5. Pursue the story source, NPC handoff, item, capability, training, or capstone that could change it.

Do not keep testing the blocker.

### Memory Failure

If memory causes a loop:

1. Pause gameplay.
2. Identify the stale attractor.
3. Correct JSON memory.
4. Compact contradictory fields.
5. Preserve one proof fact.
6. Set the corrected next prompt.
7. Validate.
8. Resume from the active prerequisite.

## Validation

Use these checks after skill or harness changes:

```bash
python3 $CODEX_HOME/skills/.system/skill-creator/scripts/quick_validate.py $CODEX_HOME/skills/crystal-llm
node --check $CODEX_HOME/skills/crystal-llm/scripts/poke.mjs
poke status
poke route
poke observe
poke observe --save-images $CODEX_HOME/pokecrystal/mcp-images/verify
```

If Next returns `500` with `Unexpected end of JSON input` during validation or harness diagnostics:

1. Restart the service.
2. Retry with slower one-call-at-a-time direct HTTP.
3. Log it in:

```bash
$CODEX_HOME/pokecrystal/learnings/ERRORS.md
```

if it affected the run.

Do not apply this restart procedure during scheduled gameplay unless the user explicitly changes the task to harness diagnostics.