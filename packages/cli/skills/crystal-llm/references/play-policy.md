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
- Treat dialogue, NPC movement, rewards, and blocker changes as story-state evidence. Do not mash through text without preserving what it means.
- Treat destination direction as intent, not immediate input.
- Pathfind from the current live topology. Use the full observed map when the viewport is too small, then identify connected floor, chokepoints, branch ends, trainers/NPCs, item pockets, and warps for this map only.
- Before choosing a staircase, door, warp, item, or NPC from a full-map render, identify the player’s connected component. A listed warp/hotspot elsewhere on the same map is not reachable evidence if walls, counters, ledges, or partition rows separate it from the current floor component.
- Scan for open gaps before declaring a route blocked or choosing a distant coordinate target. Read route rows and columns as connected floor lanes; look for breaks in wall/counter/ledge bands where `.` floor continues through. In mazes and partitioned rooms, prefer connected-lane reasoning: follow the open lane to its extreme, turn through the visible gap, continue along the next open lane, and observe at each bend, blocker, or transition.
- Do not generalize a map shape from one area to another. A shape clue is local evidence, not a reusable route model.
- In dungeons and story areas, keep pushing through reachable branches until the objective ledger has a real capstone or survival state forces a retreat. Do not stop after a partial room, first trainer, or convenient warp.
- Warps are leads, not completion proof. Completion requires a story-state change, blocker change, reward/key text, boss/trainer clear, or verified access to the next objective.
- Challenge reachable trainers that occupy or guard the current objective path unless HP, PP, status, or party state makes the fight reckless.
- Use one-tile moves around ledges, fences, rocks, trees, NPCs, grass pockets, and branch points.
- If the same movement fails twice, treat it as a bad model/tactic choice and branch.
- Treat `d`, `l`, and `r` as one-way ledges, not generic floor.
- Do not treat direction-order churn, repeated direction pivots, or same-tile pushing as strategy. They are failure patterns, not future guidance.
- On stalls, use this sequence: `poke.mjs status` -> `poke.mjs observe` with a real image -> identify the bad tactic -> make a different deliberate move -> re-check.
- Route memory and live map context beat inferred generic lessons. In Cherrygrove, the live goal is Mr. Pokemon + Mystery Egg, not stall recovery.
- In Cherrygrove, do not repeatedly push north after it fails. Use visible town context, move west through town toward the Route 30 approach, interact with NPCs when useful, and keep pursuing progress.
- If a route is not visible in the current viewport, say that. Do not overclaim route impossibility.
- Prefer roads/floors at low HP only when they actually reach the goal. If grass is the only visible route to recovery, town, supplies, a Pokemon Center, a gate, or the current objective, cross the grass anyway and handle any encounter from the live battle state.

## Subgoal Completion Tracking

Use symbolic completion tracking for every story objective:

- Define the objective in story terms, not just coordinates.
- Name the expected capstone: an NPC arrival, story speech, reward, badge, item, boss defeat, guard moving, route opening, gym access, or changed dialogue.
- Track current evidence from live text, recent events, screenshots, visible NPC locations, defeated trainers, and tested blockers.
- Track what still blocks the next objective.
- Choose the next proof step that would confirm completion or expose the remaining work.

Do not mark a story subgoal complete because the player entered an area, beat one encounter, reached a partial room, or returned to town. If a gym, route, or story gate remains blocked afterward, that is evidence the prerequisite is unfinished. Return to the relevant story area, inspect unvisited lanes, talk to relevant NPCs, fight remaining trainers, and pursue the capstone.

NPCs and story hints outrank generic navigation guesses. When a story character appears, moves, comments on the situation, gives a reward, or redirects the player, stop and update the subgoal ledger. The game is usually telling you what just changed and what still needs doing.

## Battles And Training

Experience matters. The team has been underleveled, so battle posture should be more gung ho than conservative. Do not avoid all wild battles by default; ordinary wild encounters are often useful training while moving toward the story goal.

Party development matters during active training blocks. Do not level only one Pokemon. Rotate safe EXP and change the lead when it is efficient. Switch training is optional and situational; do not let it consume runs that should be used for recovery, supplies, gym attempts, route progress, or story progress. Use the strongest Pokemon as a safety valve for dangerous fights, not as the default recipient for every routine encounter.

Catching Pokemon is useful main-story preparation when it is safe and resources allow it. Use a catch 'em all posture: try to catch new species, reasonable dupes with useful roles, and Pokemon that add party depth, HM utility, type coverage, or backup strength without derailing the current flow goal. When visiting towns, use the mart more proactively if money and bag space allow; buy more Potions and Poke Balls or better balls than a bare-minimum survival plan would. Preserve enough money for emergency survival, but prefer leaving town with enough Balls for multiple capture attempts and enough Potions to extend routes without constant Pokemon Center retreats. When the game offers a nickname prompt after receiving or catching a Pokemon, prefer giving that Pokemon a short nickname unless doing so would block urgent progress.

- Fight reasonable wild Pokemon when the active Pokemon has comfortable HP and usable PP.
- If live party evidence shows the team is underleveled for the current obstacle, actively seek experience instead of treating training as incidental. Prefer reachable trainer fights, then efficient nearby wild grass or other encounter areas, and keep going while fights produce useful EXP without derailing urgent recovery or supplies.
- Training still matters during travel. Fight ordinary wild Pokemon when the active battler or a reasonable switch can win without burning the run down. Do not flee just because HP is imperfect, the matchup is mildly bad, or the fight takes a few turns.
- During recovery, supply runs, gym routing, or story progress, wild battles are optional hazards when they become a bad trade. Run when live evidence says the fight is likely to wipe the party, severely drain PP/items, has no good available matchup, or is delaying the current objective. If Run fails, choose the next live-safe action: try Run again, switch, use an item, attack, or accept whiteout only if that is the best recovery path.
- Seek wild EXP deliberately when the party is behind nearby trainers, recent battles were close, or the next gym/story fight looks risky.
- Build backups when the area is safe enough; prefer giving routine KOs to the lagging usable teammates and reserve the carry for danger.
- Prefer continuing, switching, or using a modest item over walking back to a Pokemon Center just to top off HP.
- Do not visit a Pokemon Center by routine. Go back when there is a concrete survival reason: red HP, repeated near-KOs, badly depleted PP, dangerous status for travel, multiple fainted Pokemon, or an important upcoming fight.
- Do not require a Pokemon Center before entering grass if the current route to the Pokemon Center, shop, town, gate, or objective itself crosses grass. Move through the grass and accept the encounter risk.
- Run, heal, or retreat when HP is genuinely unsafe, not merely imperfect.
- If Ryan explicitly allows whiteouts for training, EXP preservation can make a reset acceptable.
- Do not chain red-HP encounters without a deliberate heal/reset/whiteout decision.
- In trainer and dungeon battles, do not blindly press confirm through turns. Read the battle state and choose a deliberate move, switch, item, or ball. Prefer strong STAB or type-advantaged attacks when they are available.
- Use a more aggressive action tone without changing the player name or identity: push rooms to completion, fight route blockers, and spend items to keep the objective moving when safe.

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
