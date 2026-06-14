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

2. Use the stock compact tools first. They wrap direct HTTP and return compact text/TUI JSON by default:

```bash
poke status
poke observe
poke route
poke route --tiles
poke move left
poke context
```

Default `poke observe` and `poke route` are no-image. Request images when the user asks for visual proof, when TUI/route text is insufficient, or when navigation is going poorly: `poke observe --image`, `poke observe --save-images <dir>`, `poke route --image`, or `poke proof <label>`. If repeated movement fails, the path loops, a target remains missed after several moves, or the user corrects the route shape, capture and inspect a PNG route or observe render before continuing. Use `scripts/mcp_call.mjs` when a raw MCP tool call is needed. Direct automation uses these Crystal LLM direct-HTTP scripts, not `mcporter`.

3. Use `mcporter` only for explicit harness diagnostics when a Codex-local config is available, not for scheduled gameplay. If `mcporter` says `Unknown MCP server 'pokecrystal'`, use the direct HTTP scripts; do not infer that the game/MCP is down.

## Reference Files

- Read [references/mcp-harness.md](references/mcp-harness.md) for service management, direct HTTP calls, screenshot capture, `mcporter` config, and harness failure triage.
- Read [references/play-policy.md](references/play-policy.md) before substantial gameplay, route recovery, training, or memory updates.
- Read [references/ryan-blog.md](references/ryan-blog.md) before any scheduled play/post cycle for Ryan.

## Live Play Loop

Use this loop for manual and autonomous play:

1. Call `poke status`.
2. Reconcile the live next objective against route memory, recent events, and the last observed blocker before moving. Treat the status/flow next objective as a candidate, not an order. If memory or live evidence says the candidate objective is gated by a missing item, badge permission, party capability, story flag, NPC reward, or unresolved prerequisite, make that prerequisite the immediate objective and record why.
3. If in dialogue, menus, battle, or a naming screen, read and preserve the meaning of the text before clearing it. Dialogue is gameplay evidence, not noise.
4. Call `poke observe` before terrain, menu, and battle decisions. It returns TUI text by default; prefer that over screenshots for menu rows, cursors, party order, battle allies/enemies, HP, EXP, and prompts.
5. Move one tile at a time near trees, ledges, fences, grass pockets, NPCs, or route branches.
6. If the same move fails twice, treat it as a model/tactic failure; observe, identify why the tactic was bad, and choose a different lane.
7. Use `recent_events` after surprising movement, map transitions, story dialogue, NPC movement, or tool errors.
8. Write the lesson if a route assumption, UI trap, or harness failure mattered.

Do not use repo files, ROM data, save internals, emulator memory, implementation coordinates, or generated map data to decide gameplay. For gameplay, use only live MCP outputs, real screenshots, visible text, recent in-game events, controller inputs, and the user's directions. File access is allowed for skill/runner/memory maintenance, not for in-game navigation.

## Local-Minimum Guard

Never let a proven checkpoint become the objective. If recent runs keep returning to the same nearby proof, room, NPC, menu, obstacle, route edge, or public-post theme without advancing the larger story objective, treat that as a local-minimum failure and break it immediately.

- A completed proof is not progress when repeated. Do not re-verify a completed prerequisite, re-enter a solved room, retalk exhausted NPCs, or publish another post about the same proof unless the user explicitly asks or fresh live evidence contradicts the completed state.
- When stuck in a repeated objective loop, retire the memory or prompt wording that caused the loop before playing more. Leave one compact durable fact, then set the next objective to the larger unfinished story task.
- Prefer forward progress through unvisited route branches, current blockers, healing/supplies needed for the live route, and actual obstacle use over nearby proof checks that feel safe.
- Public journals must not reward local minima. A repeated verification, repeated blocked edge, or repeated room return should be summarized as stale and retired in private notes, not turned into another milestone post.


## Long-Horizon Objective Guard

Keep a compact trajectory ledger for the playthrough's current hard story gate.

- The trajectory ledger outranks short-term route labels when live evidence proves the label is only a blocked gate name. A progress tracker can name the hard gate, but the immediate action must be the nearest unfinished prerequisite that can unlock that gate.
- Before any scheduled play, reconcile: hard gate, active prerequisite, last proof that the prerequisite is incomplete, stale local attractors, and next proof step. If the live state is sitting at a stale local attractor, leave it unless fresh story text changes the evidence.
- Do not let repeated local proof become the objective. Three posts, summaries, or runs about the same refusal, shop, counter, shore, NPC, sign, door, or route edge are enough to retire that proof and force a trajectory-level pivot.
- A useful run must either advance the active prerequisite, gather new evidence about that prerequisite, or move toward the next proof step. Re-checking a known refusal or re-entering a solved building is not progress.
- When a gate is blocked by missing story authorization, request text, NPC handoff, or capstone dialogue, pursue the story source that creates that authorization. Do not keep testing the blocked service, clerk, route, or obstacle until that story source is verified.
- Preserve trajectory memory in generic fields: hardGate, activePrerequisite, staleAttractors, lastBlockingProof, nextProofStep, and completionProof. Compact or remove older notes whose currentObjective still names a retired local attractor. Before more gameplay, apply the Memory Enhancement Procedure in the Memory Contract so the correction is visible through canonical memory, not only this skill text.

## Live Fact-Finding And NPC Clues

Use this protocol when the task is to find a clue, verify whether an NPC says something, identify where a reward/hint comes from, or debug a claim that seems to get better only after restart.

- Treat fact-finding as an evidence-gathering run, not a route assumption. Build a small checked list in the run notes: candidate map/house/cafe, NPC approached, exact dialogue evidence, whether it proves the fact, and next unchecked lead.
- Keep an `npcLedger` for clue/reward/story searches. For each NPC or overworld person checked, record: map, approximate location or local landmark, approach evidence, exact dialogue meaning, reward/item/HM/TM/flag if any, whether the clue is resolved, and the next follow-up it implies.
- Keep a `buildingLedger` for towns, routes with houses, gates, caves, shops, centers, and special venues. For each visible or remembered warp/building, record: map/local landmark, whether entered, notable NPCs inside, clues/rewards found, unchecked NPCs/rooms, exit/return evidence, and whether the building is exhausted for the current objective.
- When a route objective is blocked by a missing capability, item, story flag, or unknown clue, prioritize unchecked NPCs and buildings in the relevant town/route/nearby hub before repeating travel to the blocker. A stale flow objective is not enough reason to skip local clue exploration.
- Start from live state with `poke observe` and use only live map labels, hotspots, dialogue text, recent events, and controller inputs. Do not use repository scripts, map data, ROM source, save internals, or external spoilers to answer the in-game fact.
- Talk to candidate NPCs one at a time. After pressing `A`, immediately observe and preserve the dialogue text before advancing it. If a dialogue has multiple pages, advance one page, observe again, and record the follow-up text before clearing.
- A plausible but different clue is not completion. If an NPC gives a different reward or generic regional text, record it as checked and continue to the next live candidate rather than stopping.
- If a lead says "house or cafe near/west/east of town," inspect all live nearby warps and outdoor NPCs in that region before concluding the clue is absent. Use `poke route` or `poke observe --image` when the viewport makes building entrances or paths ambiguous.
- Do not mark a hub exhausted after checking only the obvious outdoor path. If the objective lacks a clue/reward/capstone, inspect reachable buildings, side rooms, counters, and NPC clusters unless survival state makes that unsafe.
- When the user reports a bug that appears after a while, keep using the same live session for repeated dialogue, map transitions, and menu boundaries long enough to exercise the suspected stale state. A restart alone is not proof. If the stale-state bug reappears, capture the current surface, recent events, and exact failing input before debugging or restarting.
- Finish with evidence, not confidence: report the exact dialogue or state change that proves the fact, plus any nearby candidates checked that did not prove it.

## Historical Memory Use

Use memory as live route evidence, not background flavor.

- At the start of any route recovery, dungeon, gym maze, or repeated failure, call `poke context` and match the live map/coords to `routeMemory[mapName].regionMemory` before choosing a tactic.
- Summarize the applicable remembered landmarks, working paths, dead frontiers, unvisited leads, current hypothesis, and next prompt in the run plan. If the live state falls inside an `appliesWhen` or `bounds` range, treat that entry as the current map-local playbook.
- Combine the live next objective with memory instead of blindly following either one. The correct immediate objective is the nearest proven prerequisite that advances the larger story objective: current target, missing capability, remembered NPC/reward, known item lead, or route blocker resolution. If the flow objective points through a route that memory says is impossible without a missing capability or story state, stop routing to that blocker and pursue the remembered prerequisite.
- Keep a short objective reconciliation note in run notes or memory when changing course: `flowObjective`, `memoryBlocker`, `requiredPrerequisite`, `liveEvidence`, and `nextProofStep`. Update it whenever live text, rewards, party capabilities, or blockers prove the hypothesis wrong or complete.
- When a prerequisite, room lock, NPC search, building search, or handoff objective is completed, reconcile memory in both directions. Do not merely add a newer proof note. Delete or compact every contradictory memory entry that would send a later agent back to the completed prerequisite. This includes `routeHints`, `objectiveReconciliation`, `npcLedger.currentObjective`, `npcLedger.uncheckedLeads`, `buildingLedger.currentObjective`, `buildingLedger.uncheckedLeads`, `buildingLedger.activeHandoff`, `buildingLedger.roomLock`, and matching `routeMemory[mapName].regionMemory` objectives. A completed prerequisite should leave behind one compact proof fact in the appropriate ledger, plus the next live step for the larger objective. Avoid active "do not repeat" warnings unless the user explicitly asks for one; stale warnings can become accidental objectives.
- Before following any urgent memory directive such as "do not leave", "return to", "must obtain", "unchecked lead", or "active handoff", scan nearby memory for later-dated completion proof that contradicts it. Later live proof beats older handoffs. If a contradiction exists, update the stale directive first, then proceed from the completed state.
- Use `npcLedger` and `buildingLedger` as first-class memory alongside route memory. Before concluding a prerequisite is unknown or impossible, review unchecked NPCs/buildings in the current hub and any remembered clue hubs. Before re-entering a blocked route, identify the specific checked clue or building evidence that justifies doing so.
- Apply remembered blocker solutions before inventing a detour. If memory says a prompt, item, NPC interaction, warp, or menu action solves the blocker, verify the live blocker and use the solution from observed UI before routing around it.
- Never re-test a remembered dead frontier unless live evidence shows the map state changed. Mark only the immediate edge or object as dead; do not generalize one blocked tile into an impossible route.
- Prefer remembered working paths as landmark sequences, then re-derive the next few inputs from live `observe`, `route`, and `route --tiles`. Do not replay old blind button strings.
- After a remembered route works, fails, or changes, update `routeMemory` with proof: current map/coords, target landmark, tested edge, reason, and the live evidence that confirmed it.

## HM Capability Check

Keep a compact `hmCapabilityLedger` only for observed HM ownership, known user, and badge usability. Check it when the current route is blocked by an HM-style obstacle or when the run just received/taught an HM. Do not run broad clue searches or repeat status checks solely because an HM exists; use live party/menu evidence only when it affects the immediate blocker.

When an HM is proven owned or usable, compact memory instead of accumulating warnings. Preserve one capability fact in `hmCapabilityLedger`, then remove stale "missing HM" objectives, old handoffs, room locks, NPC/building searches, unchecked leads, route hypotheses, and route-memory entries whose purpose was obtaining that HM. Do not leave repeated "do not reacquire" reminders in active objective fields; those reminders become their own attractor. Keep only route memory that helps current navigation or obstacle use.

If field use fails despite ownership proof, record the live failure reason precisely, such as wrong shoreline tile, missing badge permission, wrong party member, fainted user, prompt not facing water, or move not currently in party. Treat that as an immediate obstacle diagnosis, not as a reason to reacquire the HM, unless the live bag/menu proves it absent.

## Menu Navigation Discipline

Menus, prompts, dialogue boxes, naming screens, and battle command panes are live state machines. Never navigate them from memory or from an earlier attempt. Before choosing inputs, identify the current state from the observed surface, selected row, prompt/dialogue flags, visible text, and available controls.

- Treat every modal boundary as a checkpoint. A press that clears text, opens a prompt, closes a prompt, returns to a parent menu, enters a submenu, or exits to overworld must be followed by an observation or compact TUI state check before the next navigation decision.
- Do not combine state-changing acknowledgement inputs with navigation inputs in one shell command. Dialogue and prompt advancement can land on different surfaces depending on script state.
- Use short macros only when the start state and target state are both explicit. A macro should cross one predictable boundary, such as moving from an observed menu row to another visible row, confirming the highlighted option, or backing out one menu layer.
- Derive cursor movement from the currently highlighted row, not from a memorized absolute position. If the selected row is unexpected, recalculate from what is visible.
- When an action fails or opens an error message, clear the message, observe the returned state, and recover from that state. Do not replay the original input sequence.
- For any multi-step item, ability, key item, field move, equipment, or party action: verify each intermediate screen, choose only from observed valid options, and return to the original blocker/objective to prove the action had the intended effect.
- For PCs, counters, signs, item balls, shelves, machines, and similar objects, side-facing often does nothing. Do not keep pressing A from a side tile after a failed interaction. Use live `observe`, `interactionLane`, `interactionSetup`, or `route` to identify the usable approach tile, stand directly in front of the object, face the object, press A once, and verify that text/menu/dialogue opened before continuing.
- In battle party switching, use the battle `PKMN` command, then the live party menu. If selecting a Pokemon opens a submenu, `STATS` is the default and `SWITCH` is the next row. To switch safely: observe the cursor on the target Pokemon, press `A` to open the submenu, observe `SUBMENU`, press `down` to `SWITCH`, press `A`, then verify the active ally changed in TUI before choosing a move.

## Story Objective Completion

Track objectives symbolically, not just by movement or partial fights. For every story subgoal, keep a small completion ledger in the run summary or durable notes:

- `objective`: the actual story task in generic terms, such as clearing the current dungeon, unlocking the next gate, or opening the next major route.
- `expectedCapstone`: the kind of confirmation that should happen when it is done, such as an NPC arriving, a rescued character, a reward, a forced scene, a guard moving, a warp opening, or the next blocker disappearing.
- `currentEvidence`: dialogue, map transitions, defeated trainers, item/reward text, NPC position changes, and route blockers tested live.
- `stillBlockedBy`: any visible NPC, trainer, locked door, cut tree, or unexplored branch that still prevents the next story objective.
- `nextProofStep`: the next concrete live check that would prove completion or reveal the remaining task.
- `objectiveReconciliation`: how the live flow objective, route memory, and recent blocker evidence agree or conflict. If they conflict, name the prerequisite now being pursued and the proof that will return focus to the larger objective.
- `npcLedger`: checked and unchecked NPCs relevant to the objective, including clue/reward/status and follow-up.
- `buildingLedger`: checked and unchecked buildings/warps/venues relevant to the objective, including interior NPCs, rewards, clues, and whether each is exhausted.

Do not mark a story objective complete just because the character entered the area, beat one trainer, reached an early room, or returned to town. A story objective is complete only after a symbolic capstone or state change is verified. Examples of valid completion evidence include a relevant NPC arriving or talking after progress, hostile blockers leaving, a guard no longer blocking a gym or route, a reward/key item text, a badge/TM text, or a new warp/door becoming usable.

When an NPC or the story gives a hint, treat it as the primary route clue. If a relevant NPC appears, moves, gives a speech, or reacts to progress, stop and update the ledger before pathing. Do not downgrade explicit instructional dialogue into flavor text. If text says to use, teach, equip, show, deliver, return, buy, heal, or bring something, the next proof step must be that concrete action or a live verification that the action is impossible right now.

Do not confuse a progress-tracker gate label with the next action the client should take. Treat status/flow goals as weak labels: they name a broad gate, but they do not override NPC clues, live dialogue, durable trajectory memory, or the missing capstone proof. If status names a gate while NPC/story evidence says a prerequisite request, authorization, item, battle, training need, or route capability is missing, largely ignore the status label for action selection and pursue the live prerequisite that can change the story state. If an item, clerk, guard, counter, route, or boss refuses because the prerequisite has not been established yet, record the refusal as gate evidence and explore for the missing story capstone or prerequisite. Repeated attempts at the refused target are not progress until new live evidence changes the prerequisite state. Once live dialogue, memory, or current context proves a requested item, medicine, delivery, or handoff is available to pursue, leave the proof behind, obtain what was requested, and continue the follow-up chain.

When repeated runs prove the same refusal, blocked service, route edge, or local room, turn that proof into a route-away obligation. The next run plan must name the hard gate, the active prerequisite, the retired attractor, the nearest route-away waypoint, and the completion proof required before the attractor can be retried. Do not publish, summarize, or count another copy of the same refusal as progress. A status/flow label, current map name, or nearby visible object must not rewrite the active prerequisite until the required completion proof is observed live.

When NPC dialogue, town clues, or repeated blocker evidence identify a specific story location or capstone, make that capstone the route target and keep pushing until it is reached or live evidence proves a different prerequisite. For a tower, dungeon, or multi-floor story building, reaching the building or an early floor is not enough; keep climbing, using stairs, holes, side rooms, and trainer lanes until the top-floor NPC/capstone is observed.

Manage story progress through explicit subtasks. Keep a compact `subtaskLedger` with the current hard gate, immediate subtask, why that subtask advances the gate, proof needed to finish it, checked leads, and next unchecked lead. When the hard gate is broad, do not let the agent choose the gate label as the action. Choose a concrete exploratory subtask that can produce live evidence, then update or replace the subtask after each capstone, refusal, dead branch, NPC clue, warp transition, or route failure.

If a gym/route remains blocked after a partial dungeon clear, infer that the dungeon/story task is not done and return to find the unvisited branch, remaining trainer, boss, item, or capstone event. If a route remains blocked immediately after receiving an explicit tool/hint for that exact blocker, infer that the missing step is applying the tool/hint, not more random pathing.

## Resource And Catching Policy

- Treat catching Pokemon as useful preparation for beating the game, not as a side objective that overrides story progress.
- Catch 'em all posture: when resources and safety allow, try to catch new species, reasonable dupes with useful roles, and Pokemon that improve party depth, type coverage, HM utility, or backup strength.
- When visiting towns, use marts more proactively. Buy more Potions and Poke Balls or better balls than the old conservative policy would, while preserving enough money for emergency survival.
- Prefer leaving town with enough Balls to make multiple capture attempts and enough Potions to extend routes without constant Pokemon Center retreats.
- When the game offers a nickname prompt after receiving or catching a Pokemon, prefer giving that Pokemon a short nickname unless doing so would block urgent progress.

## Battle And Training Policy

- Training means committing to battles, not dodging them. Fight ordinary wild Pokemon and reachable trainers instead of avoiding encounters or escaping for comfort.
- Training still matters. Prefer fighting ordinary wild Pokemon when the active battler or a reasonable switch can win without burning the run down. Do not flee just because HP is imperfect, the matchup is mildly bad, or the fight takes a few turns.
- In wild battles during recovery, supply runs, gym routing, or story progress, do not be afraid to run when live evidence says the fight is a bad trade: likely wipe, severe PP/item drain, bad type matchup with no good switch, multiple fainted teammates, or the fight is blocking the current objective. If Run fails, choose the next live-safe action: try Run again, switch, use an item, attack, or accept whiteout only if that is the best recovery path.
- Low HP is not a reason to retreat to town, leave grass, or abandon active training. Treat fainting and whiteout as acceptable training costs, then continue from the resulting live state.
- Do not require healing before crossing grass when grass is the only visible route to recovery, town, supplies, a Pokemon Center, or the current objective. Walk into the grass, accept the encounter risk, and use live battle decisions, items, switches, attacks, or whiteout recovery if a fight happens.
- Treat money as a tracked training resource when it is visible or directly relevant to the current choice.
- When the run is corrected for rushing, being too objective-focused, or needing training, treat that as an explicit override: stop the badge/story push, leave the gym or capstone route if needed, and make trainer hunting, wild EXP, catches, supplies, and party development the active goal until live evidence shows the team improved.
- Training is not only incidental travel work. Deliberately explore nearby routes, side buildings, grass, and reachable NPC/trainer lanes to find missed trainer fights and safe EXP before retrying a wall such as a gym leader.
- Training means earning EXP and levels across the team, not just powering one lead. Default to real battles that produce at least one verified level gain for an underleveled or neglected party member when feasible.
- When a boss, gym leader, route, or user correction says the team is not viable, make balanced team readiness the active goal. Set an approximate target level range from the live obstacle and local memory, then keep training until multiple usable non-starter teammates are near that range, not merely until the starter can carry harder.
- For training blocks, inspect the live party before and after; verify changed levels, HP, status, and any move-learning text from live UI.
- If live party evidence shows the team is underleveled for the current obstacle, actively seek experience instead of only incidental travel. Look for reachable trainer fights first, then efficient nearby wild grass or other encounter areas, and keep training while it produces useful EXP without derailing urgent recovery or supplies.
- Use wild grass deliberately for experience when the team is behind the area, the next trainer/gym looks risky, or recent fights were close. Short training loops are useful progress, not a stall, when they produce EXP.
- Train the whole usable party when training is the active goal. Prefer direct KOs, reasonable lead rotation, and efficient story-relevant battles over prolonged setup. Switch training is optional and situational; do not let it delay supplies, healing, route recovery, gym attempts, or other main-story progress.
- For switch training, only use it when the live goal is explicitly training and the setup will not waste the run. The weak Pokemon gets EXP for starting or participating, not for attacking. Put the weak Pokemon first before entering grass or a trainer line, verify the battle opens with that Pokemon as `ALLY`, then switch immediately to the strong safety Pokemon before selecting any attack. After the battle, verify the weak Pokemon's EXP/level from TUI stats; do not count the attempt as successful if EXP did not change.
- Treat non-starter development as the priority during training blocks. The starter may finish dangerous fights, but routine wild encounters, rematch preparation, and safe trainer KOs should build the rest of the team first.
- Before retrying a known wall after a training override, verify party progress from live UI or battle evidence: levels gained, stronger non-starter moves, healthier team distribution, or several non-starter KOs. Do not return just because the starter gained another level.
- Use the strongest Pokemon as the safety valve for dangerous fights, not as the default recipient for every ordinary trainer or wild encounter.
- Do not waste tempo switching out a 1 HP or near-faint active Pokemon when the opponent is likely to move first or KO it anyway. Let the active Pokemon attack if it can meaningfully damage, debuff, or finish the foe; otherwise let it faint and choose the replacement from the free switch prompt. This preserves the incoming Pokemon's first actionable turn instead of spending it on a manual switch.
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
- Grass is passable route terrain, not a blocker. When the live route to healing, supplies, town, a gate, or the current objective requires crossing grass, cross it one tile at a time. Do not stand outside grass waiting for a heal-first plan that the map does not currently offer.
- On multi-floor or multi-warp maps, maintain a navigation ledger before moving: current map/coords, current objective, intended next warp/landmark, last useful warp used, local failed moves, and a recovery move if the next lane fails.
- Treat `route --no-image` as the floor-level warp graph when the viewport is too local. Identify the next target warp or landmark from the live render, then move through visible lanes toward it in small verified chunks.
- A blocked tile only invalidates that immediate tile, not the floor objective. After two blocked moves in a pocket, observe, call `route --no-image`, return to the last useful warp or choose a different visible lane, and continue unless HP, battle, or UI state makes movement unsafe.
- When an NPC or trainer blocks a direct line to a target warp, interact or fight if appropriate; if dialogue only or still blocked, route around using adjacent visible lanes instead of ending the run.
- When a major route or gym remains blocked, do not keep testing that blocker. Use it as a symbolic failure signal: identify the unfinished prerequisite, update `stillBlockedBy`, and return to the relevant story area until the capstone state change occurs.
- Do not claim a route is impossible from a single viewport; say "not found in the observed viewport" unless a larger scouted chart proves it.
- Capture start/end proof images when the user asks for screenshots or when a run is being used as evidence.
- Use PNG renders earlier when struggling. If the live route is confusing, the same area is revisited, the model is making bad spatial assumptions, or the user says to use images, call `poke route --image` or `poke observe --image`, inspect the saved PNG, and base the next few moves on that visual map.

## Stock Tools

Prefer the `poke` shortcut for these compact wrappers to reduce prompt and output tokens. If `poke` is not on `PATH`, use the bundled `scripts/poke.mjs` path from this skill installation, or the local full path `$CODEX_HOME/skills/crystal-llm/scripts/poke.mjs`.

```bash
poke status
poke observe
poke observe --image
poke observe --grid
poke route
poke route --image
poke route --tiles
poke proof cherrygrove
poke move left
poke press A
poke clear
poke events
poke context
```

Use `context` instead of reading the full learning-state JSON when you only need the current map, next prompt, and route memory. Use `route` when the Game Boy viewport is too local for navigation; it renders the full current map as text/grid data without adding pathfinding or scripted movement. Add `--image` or `--tiles --image` only when a visual full-map render is needed. Use `observe --grid` when viewport text terrain is needed; plain `observe` returns compact TUI text and does not save images.

Use TUI and route views together: TUI/`observe`/`observe --grid` is best for exact text, selected menu rows, prompts, battle state, active ally/enemy, EXP, and local obstacle tokens; `route` is best for spatial layout, connected floors, warps, ledges, water, and tree clusters.

## Memory Contract

Route-specific corrections must live outside this reusable skill.

- Canonical state: `$CODEX_HOME/pokecrystal/poke_learning_state.json`
- Human route notes: `$CODEX_HOME/pokecrystal/poke_learning_journal.md`
- Daily sidecar: `$CODEX_HOME/pokecrystal/memory/YYYY-MM-DD-pokecrystal-learning.md`
- Durable learning log: `$CODEX_HOME/pokecrystal/learnings/LEARNINGS.md`
- Tool/runtime failures: `$CODEX_HOME/pokecrystal/learnings/ERRORS.md`

### Memory Enhancement Procedure

When a correction, repeated-run failure, stale objective, or trajectory mistake is found, the skill must improve durable memory before more gameplay. Do not only add prose warnings.

1. Read the canonical state and identify the stale attractor: the exact NPC, building, route edge, menu, post theme, tracker label, or short-term objective that caused the loop.
2. Update `trajectoryLedger` when the issue is story-level: `hardGate`, `activePrerequisite`, `staleAttractors`, `lastBlockingProof`, `nextProofStep`, and `completionProofNeeded`.
3. Update `objectiveReconciliation` with `flowObjective`, `memoryBlocker`, `requiredPrerequisite`, `liveEvidence`, and `nextProofStep`. This entry must explain why the live flow label is or is not the immediate action.
4. Update `nextPrompt` so `poke context` surfaces the corrected immediate objective without needing to read old journal entries.
5. Prune or compact stale active fields that would pull the next run backward. Check and update `routeHints`, `npcLedger.currentObjective`, `npcLedger.uncheckedLeads`, `buildingLedger.currentObjective`, `buildingLedger.uncheckedLeads`, `subtaskLedger`, and any `routeMemory[mapName]` entries whose `preferredNextWaypoint` or `unvisitedLeads` still name the retired attractor.
6. Preserve one compact proof fact for the retired attractor, then point the relevant `preferredNextWaypoint` to the active prerequisite. Avoid repeated "do not repeat" reminders unless they are paired with the concrete next proof step.
7. When the corrected prerequisite must not be displaced by stale flow/status labels, add an `objectiveLock` with the locked prerequisite, acceptable completion proof, disallowed rewrite triggers, and next proof step. Do not rewrite that prerequisite until the completion proof is observed live.
8. Append a short journal/daily-memory note only after the JSON state is corrected. The journal is supporting evidence; the JSON state is what future runs must consume.
9. Validate the edit: parse the JSON, run skill validation, and inspect the corrected fields. If `poke context` is available, verify that it exposes the new `nextPrompt`; if the MCP server is timing out, record that validation gap.

Memory edits must enhance future decisions generically: retire stale local attractors, preserve the hard gate, and promote the nearest unfinished prerequisite that can change live story state.

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
- active trajectory reconciliation: hard gate, active prerequisite, stale attractors, route-away waypoint, completion proof, and why the status/flow label is not enough if it conflicts
- current `hmCapabilityLedger` only if the immediate blocker or newly received move makes HM capability relevant
- recent movement audit: last 5-10 movement events, net displacement, repeated blockers, dead pockets just proven, and the last useful landmark
- recent failed attempts on that map
- story completion ledger: objective, expected capstone, current evidence, remaining blocker, and next proof step
- current navigation ledger for multi-warp or maze maps: current floor, target floor/warp/landmark, last useful warp or landmark, checked/dead frontiers, failed local moves, and recovery lane
- next 2-3 tactics
- survival/training policy for the active Pokemon, including whether to seek wild EXP before the next story fight

Scheduled play should do enough real interactions to matter, usually around 30. Failed movement is a model/tactic problem, not a "blocking" excuse. It should not quit after one failed movement; it should branch, interact, inspect, and keep trying unless HP, battle state, or UI state makes further play unsafe. For story tasks, "enough interactions" means pursuing the capstone state change, not sampling the first quarter of the area and leaving. A scheduled run is not locked to one stale item or errand label: if live evidence shows the current gate needs training, supplies, healing, a different NPC clue, a route capability, a medicine pickup, or a prerequisite story event, make that concrete prerequisite the active objective and pursue it.

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
poke route
poke observe
poke observe --save-images $CODEX_HOME/pokecrystal/mcp-images/verify
```

If Next returns `500` with `Unexpected end of JSON input`, restart the service, retry with slower one-call-at-a-time direct HTTP, and log it in `.learnings/ERRORS.md` if it affected the run.
