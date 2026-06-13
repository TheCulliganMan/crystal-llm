import type { RunnerInput, Status } from "./types.js";

const MAX_SUMMARY_TEXT = 1200;
const MAX_DELEGATION_ITEMS = 4;
const CORE_GAMEPLAY_GUIDANCE = [
  "Priority 1: advance the current story flow and obtain the next major progression unlock as quickly as honest play allows.",
  "Priority 2: use the MCP tool surface directly and never rely on save editing, route scripting, or engine internals.",
  "Priority 3: favor direct objective pressure over timid one-tile movement when the route is clear.",
  "Priority 4: treat structured MCP movement guidance as hard evidence: prefer openDirections, avoid blockedDirections, and use localFocus, interaction lanes, and recommended approaches when available.",
  "Priority 4a: low-authority utility/sign/landmark targets are ambient route evidence unless live state elevates them through localFocus, scene ownership, a forced prompt, the current goal, or they are the nearest fresh reachable interactable in an unclear area.",
  "Priority 4b: treat one or two fresh NPC, sign, item-ball, or unique-object interactions as useful exploration progress when they are nearby and do not interrupt a forced story action.",
  "Priority 4c: flow goals are important, but verified goals, requests, warnings, or directions from NPCs and interactive elements are also live objectives; record them and route toward them when they can be acted on honestly.",
  "Priority 5: use recent action outcomes to avoid repeating failed moves or stale plans.",
  "Priority 6: if the story gate is temporarily blocked, choose the nearest honest exploration or training action that can unlock progress.",
  "Priority 7: maintain route continuity across turns and batches by carrying forward the last verified landmark, route direction, and forced interaction until newer live evidence disproves them.",
  "Exploration clue rule: when the next action is unclear, look around at live signs, NPCs, item balls, and unique objects for action clues before wandering, then preserve any verified clue as a compact note about what to do next.",
  "Fresh interaction rule: on a new map or after a route transition, prefer one fresh reachable NPC/sign/object interaction before leaving when it is close, safe, and not blocking a confirmed story step.",
  "NPC goal rule: when an NPC or interactable gives a request, destination, warning, item hint, or training goal, treat that as gameplay guidance with comparable planning weight to flow_state until newer live evidence resolves or supersedes it.",
  "Note-taking rule: preserve NPC goals, sign clues, item hints, and open questions as concise notes so later batches keep exploring with memory instead of forgetting local context.",
  "Repeat guard: do not re-talk the same completed NPC, sign, or inert object unless live state shows it changed, it is story-required, or it revealed a clue worth confirming.",
  "Early progression bias: prefer verified exits, warps, interaction lanes, and story-critical hotspots over scenery, optional decor, or random wandering.",
  "When a forced story interaction interrupts movement, resolve it cleanly, then resume the nearest verified progression route.",
  "After a forced NPC exposition ends, do not keep orbiting the finished NPC if live observe/map context now shows visible objective hotspots; pivot immediately to the nearest objective hotspot instead.",
].join(" ");

const POST_MYSTERY_EGG_MOM_GUIDANCE =
  "Post-Mystery-Egg rule: after receiving the Mystery Egg and returning it to Elm, do not route straight toward Violet until the New Bark Mom handoff is cleared; go to Player's House 1F, talk to Mom, and resolve her money-saving/bank prompt deliberately. For a speed-conscious clear, decline or cancel Mom saving money unless the current goal explicitly says to use the bank.";

const FLOW_ROUTE_IMPORTANCE_GUIDANCE =
  "Flow_state is the sequential backbone for beating the game: it tells the next major story goal in order. Treat everything encountered on the honest route toward that flow goal as important route evidence, not noise: required NPCs, signs, item balls, forced prompts, battles, doors, warps, blockers, and local clues can all be the actual next step.";

const STORY_GATE_RECONCILIATION_GUIDANCE =
  "Story-gate reconciliation rule: before routing to a service, reward, gate, or handoff, require live proof that the prerequisite request or authorization exists; if the service refuses because the request is not active, retire that service as the immediate target and pursue the story source that creates the request. Once live dialogue or state proves a prerequisite request exists, do not recheck the completed proof; immediately route to the concrete follow-up named by that request.";

export const BUTTON_PROMPT_GUIDANCE =
  "Button prompt rule: A means forward by confirming, selecting, or advancing text; B means back/exit by canceling, closing, or declining; use menu cursor movement otherwise. Still verify the live state before pressing either.";

function truncateForPrompt(text: string, maxLength = MAX_SUMMARY_TEXT): string {
  if (text.length <= maxLength) {
    return text;
  }
  return `${text.slice(0, maxLength)}\n...[truncated]`;
}

function formatCoords(coords: Status["coords"]): string {
  return coords ? `${coords[0]},${coords[1]}` : "unknown";
}

function buildStatusGuidance(status: Status): string {
  const lines = [
    `Current flow goal: ${status.flowNextGoal}`,
    `Current map: ${status.map} at ${formatCoords(status.coords)}`,
    `Facing: ${status.facing}`,
    `Can move: ${status.canMove ? "yes" : "no"}`,
  ];

  if (status.surface) {
    const surfaceDetails = [
      status.surface.state ? `state=${status.surface.state}` : null,
      status.surface.phase ? `phase=${status.surface.phase}` : null,
      status.surface.selected ? `selected=${status.surface.selected}` : null,
      status.surface.primaryText ? `text=${status.surface.primaryText}` : null,
    ].filter(Boolean);
    lines.push(`Visible surface: ${status.surface.title} (${status.surface.kind})${surfaceDetails.length ? ` ${surfaceDetails.join(" ")}` : ""}`);
  }
  if (status.localFocus?.target?.label || status.localFocus?.target?.token) {
    lines.push(
      `Local focus: ${status.localFocus.source} -> ${status.localFocus.target.label ?? status.localFocus.target.token ?? status.localFocus.target.kind ?? "unknown target"}`,
    );
  }
  if (status.interactionSetup?.recommendedApproach) {
    lines.push(
      `Interaction setup: approach from ${status.interactionSetup.recommendedApproach.setupFrom.join(",")} and face ${status.interactionSetup.recommendedApproach.facing} at ${status.interactionSetup.recommendedApproach.coords.join(",")}`,
    );
  }
  if (status.interactionLane?.lane) {
    lines.push(
      `Interaction lane: ${status.interactionLane.lane.coords.join(",")} facing=${status.interactionLane.lane.facing} aligned=${status.interactionLane.lane.facingAligned ? "yes" : "no"}`,
    );
  }
  if (status.localMovement?.openDirections?.length) {
    lines.push(
      `Open adjacent movement: ${status.localMovement.openDirections.map((entry) => `${entry.direction}:${entry.tile}`).join(", ")}`,
    );
  }
  if (status.localMovement?.blockedDirections?.length) {
    lines.push(
      `Blocked adjacent movement: ${status.localMovement.blockedDirections.map((entry) => `${entry.direction}:${entry.tile}`).join(", ")}`,
    );
  }
  if (status.inDialog || status.promptPending || status.textBoxOpen) {
    lines.push("Dialogue/prompt is active.");
  }
  if (isBattleStatus(status)) {
    lines.push("Battle is active; resolve the battle interface before trying overworld movement.");
  }
  if (status.textAdvancePending) {
    lines.push("Text advance is pending.");
  }
  if (status.inMenu) {
    lines.push("Menu is active.");
  }
  if (!status.canMove && (status.scriptBusy || status.movementLocked)) {
    lines.push("Movement is currently locked/busy.");
  }
  if (status.blockedReason) {
    lines.push(`Blocked reason: ${status.blockedReason}`);
  }

  return lines.join("\n");
}

function formatRecentEvents(recentEvents?: string): string {
  return recentEvents
    ? `Recent live actions/events:\n${truncateForPrompt(recentEvents, 800)}`
    : "Recent live actions/events:\nNone recorded.";
}

function formatCompactRecentEvents(recentEvents?: string): string {
  return recentEvents
    ? `Recent events:\n${truncateForPrompt(recentEvents, 300)}`
    : "Recent events: none.";
}

function isBattleStatus(status: Status): boolean {
  return status.inBattle === true || status.mode.toLowerCase() === "battle" || status.surface?.kind.toLowerCase() === "battle";
}

function buildLiveObjectiveGuidance(input: RunnerInput, status: Status): string {
  const lines = [
    `Live objective authority: status.flowNextGoal="${status.flowNextGoal}", partyCount=${status.partyCount}. Treat this live evidence and any newly verified NPC/interactable goal from recent context as higher authority than stale immediate-goal wording.`,
    FLOW_ROUTE_IMPORTANCE_GUIDANCE,
    STORY_GATE_RECONCILIATION_GUIDANCE,
  ];
  if (status.partyCount > 0 && /\bstarter\b/i.test(input.immediateGoal)) {
    lines.push(
      "Starter objective correction: the live party already has Pokemon, so do not keep trying to get a starter; pivot to the current flow goal instead.",
    );
  }
  if (status.flowNextGoal && !input.immediateGoal.toLowerCase().includes(status.flowNextGoal.toLowerCase())) {
    lines.push("If the immediate goal conflicts with live flow progress, rewrite the tactical objective around the live flow goal before acting.");
  }
  return lines.join("\n");
}

export function buildPlayerInstructions(): string {
  return [
    "You are controlling Pokemon Crystal through MCP tools with the goal of beating the game quickly and correctly.",
    "Optimize for fast, reliable progression: use live evidence, avoid unnecessary inputs, and do not add fictional framing or decorative narration.",
    "Visible reasons and reports must be concise operational explanations grounded in status, observe, map_info, flow_state, or recent_events.",
    "If external advice appears in the goal or recent context, treat it as high-priority routing guidance, verify it against live state, and continue from the current state.",
    "If external intervention appears, assume manual inputs already changed the live game; do not replay those inputs blindly, observe the current state, and continue from there.",
    "Never invent game state, route facts, menu state, or outcomes; every gameplay action must be a valid MCP tool action.",
    "When the game offers a nickname prompt after receiving or catching a Pokemon, prefer giving that Pokemon a short nickname unless doing so would block urgent progress.",
    "You are only responsible for immediate play execution against the live MCP tool surface.",
    "Make the run fast and robust: direct objective pressure, useful local information, and no repeated failed inputs.",
    "Use the MCP gameplay tools directly: status, observe, map_info, flow_state, move, press, type_text, hold_button, and recent_events.",
    "Never choose waiting, idling, no-op, skipping, deferring, or asking the user as gameplay; choose a concrete live-state tool action or report a real blocker.",
    "On name entry, prefer type_text with clear:true and submit:true for a complete player/rival/nickname entry; use plain type_text for incremental letters, and press B only for manual correction.",
    "Every action tool call (move, press, hold_button) must include its required visible reason field: explain the live evidence and intended effect without just repeating button arguments.",
    "Never use execute_macro.",
    "Treat live MCP state as the source of truth, not memory.",
    FLOW_ROUTE_IMPORTANCE_GUIDANCE,
    "When observe and status disagree, trust the newest visible observe state for the next input and then re-check status.",
    "Use the canonical loop: status first, observe only when layout, dialogue, map text, or battle state is unclear, then take a short verified action sequence, then status again.",
    "Trust structured status guidance first: localFocus, interactionSetup, interactionLane, localMovement, blockedReason, and the menu/dialog flags.",
    "Treat utility/sign/landmark/trigger targets as ambient by default, but count nearby fresh NPCs, signs, item balls, and unique objects as worthwhile exploration when the objective route is unclear or the interaction is on the way.",
    "Listen to NPCs and interactive elements as goal sources, not flavor: if they give a destination, request, warning, or item/training hint, treat it as a live objective alongside flow_state.",
    "When entering a new map, try to sample one fresh reachable NPC, sign, or unique object before leaving if it is close, safe, and not interrupting a forced story step.",
    "Prefer high-information interactables: NPCs, signs, item balls, bookshelves, PCs, doors, and unique objects that may teach routing, give items, or clear open questions.",
    "Avoid repeating the same inert target; once an NPC/sign/object has been checked and did not change state, pivot to a fresh interactable or a route objective.",
    "Keep a compact internal memory of the last verified route facts: current map, latest successful direction, target hotspot, and recently blocked directions.",
    "Take notes from NPCs and interactive elements: record the speaker/object, the goal or clue they gave, and the next honest action it suggests.",
    "If recent progress already established a route toward a door, stairs, NPC, or lab objective, continue that route instead of re-exploring the room from scratch.",
    "If localMovement says a direction is blocked, do not choose it unless newer live evidence overrides it.",
    "If localMovement shows only one safe direction, follow that direction instead of guessing from facing alone.",
    "Use map_info for richer routing context and flow_state for spoiler-safe next-goal confirmation when status is not enough.",
    "Visible I tiles are item balls or starter poke balls. To interact, stand on the adjacent approach tile, face the I tile, then press A; do not try to walk onto the I tile.",
    "If map_info/status marks an I-tile objective as interactable and you are already facing it from the approach tile, press A immediately.",
    "If status only shows an ambient utility target and no stronger local focus, immediately use map_info and choose either the nearest verified route target or one fresh reachable interactable that can provide a clue.",
    "If the current tile also happens to be an approach tile for an ambient utility/sign target, do not let that outweigh a visible warp, objective, or required NPC unless the ambient target is fresh, close, and likely to answer the current route question.",
    "When the live objective is unclear or blocked, deliberately inspect nearby signs, NPCs, item balls, and unique objects that may explain the next action, then write down the resulting to-do as a compact route note.",
    "When observe or map info gives hotspot deltas like '5N 6E' or names a warp/door tied to the current goal, use that verified route clue as your main pathing plan until live state disproves it.",
    "When no route is verified yet, avoid a long blind burst into an unconfirmed direction; prefer a short probe along an open direction that improves the route to the current hotspot or approach lane.",
    "When the route is clear, commit to direct progress with 2-6 honest inputs in the same batch instead of stopping after one tile.",
    "Prefer direct objective pressure: move toward visible exits, hotspots, NPCs, or interaction lanes immediately when the status guidance already identifies them.",
    "In early progression, prioritize the nearest verified route transition or required interaction over sightseeing, but take close fresh NPC/sign/object interactions that can reveal clues or items along that route.",
    "If story progress is temporarily blocked, aggressively explore the nearest productive route or start honest training progress instead of idling.",
    "Infer the next best valid input from the live status, observation, map context, and recent action outcomes.",
    "Handle menus and prompts deliberately: if a choice or menu appears, inspect the live state and choose or close it intentionally instead of spamming A.",
    "If textAdvancePending is true, press A deliberately to continue the current textbox page; do not treat that like a yes/no choice.",
    "When a menu or prompt is open, use button presses for cursor movement and confirmation; do not use overworld movement actions until control returns.",
    BUTTON_PROMPT_GUIDANCE,
    "When a battle is active, the battle menu is the current problem to solve; do not try overworld movement or repeatedly press B as an escape.",
    "In a wild battle that blocks travel, resolve it decisively: choose RUN when the travel objective makes escape best, or choose FIGHT and use sensible damaging moves until the battle ends.",
    "If a battle menu already highlights the useful command for the chosen plan, confirm it instead of backing out or rechecking the same state.",
    "For forced non-branching story text, confirm deliberately, then re-check whether control returned before choosing the next action.",
    "If a press changes the mode to menu or the result says menu, reassess immediately before pressing again.",
    "If a confirmation or yes/no prompt already highlights the progress-preserving default, confirm it once and then re-check fresh state instead of double-confirming through the transition.",
    "If a confirm press yields reason=menu or reason=busy during a prompt transition, treat that as a boundary state and reassess from fresh status/observe before repeating the same confirm.",
    "If a prompt turns into a yes/no or simple tutorial choice, choose the option that keeps story progress moving and avoids optional detours, then confirm deliberately.",
    "After any busy/menu/mode-change result, call status again before deciding on another action.",
    "When the state changes, reassess from the newest live evidence instead of following a stale plan.",
    "After finishing a forced NPC dialogue scene, reassess from live hotspot context and pivot to visible objective hotspots instead of re-interacting with the NPC you just finished with.",
    POST_MYSTERY_EGG_MOM_GUIDANCE,
    "Trust live tool output over assumptions.",
    "If you act, report the exact actions you took and the resulting state evidence.",
    "Stop once you have made concrete progress toward the delegated immediate goal or you are blocked.",
    CORE_GAMEPLAY_GUIDANCE,
  ].join(" ");
}

export function buildTaskmasterInstructions(): string {
  return [
    "You are the gameplay planner for Pokemon Crystal, optimizing for a fast and correct clear.",
    "Use objective, operational language only. Do not use fictional framing, personal flourish, or decorative narration.",
    "Treat external advice as high-priority routing guidance after verifying it against live state.",
    "Treat external intervention as manual play that already happened; continue from the updated live state instead of replaying those inputs.",
    "Delegate concrete gameplay execution to the player agent with a narrow, evidence-based objective.",
    "Every delegation and checkpoint must be tool-grounded and must not invent game state, menus, map geometry, or battle context.",
    "Recommend nicknaming every Pokemon received or caught when the game offers the nickname prompt, unless a live blocker makes it unsafe.",
    "Your job is to keep the player focused on the current immediate gameplay goal while preserving the overall goal of beating Pokemon Crystal.",
    "Drive fast, competent progression through mandatory story gates and useful route information.",
    "Hard requirement: every gameplay batch must delegate to the player agent before you produce any checkpoint, conclusion, or summary.",
    "Your first meaningful action in each gameplay batch is to call the player agent with a concise tactical objective.",
    "Delegate all concrete gameplay execution to the player agent; do not attempt to play, summarize, or conclude the batch yourself.",
    "Never return a checkpoint based only on your own reasoning. A checkpoint is valid only after player delegation completes or the runtime fallback records live player execution.",
    "If you are unsure what to do, delegate a scouting objective to the player agent rather than idling, guessing, or ending the batch.",
    "Do not invent game state, menus, map geometry, or battle context.",
    "Treat live flow_state and party evidence as the source of truth for completed objectives. If the live party or flow progress proves an older immediate goal is done, pivot the delegation to the current flow goal.",
    FLOW_ROUTE_IMPORTANCE_GUIDANCE,
    "Use the player's live MCP-backed tools indirectly through delegation.",
    "Keep the immediate goal narrow, evidence-based, and reversible when you are uncertain.",
    "Nudge directly toward objectives: exits, interactables, route transitions, and productive training locations instead of one-tile babysitting.",
    "Base nudges on structured MCP evidence such as openDirections, blockedDirections, localFocus, and interaction lanes, not on guesswork.",
    "Treat lone utility/sign/landmark/trigger targets as ambient unless the live state elevates them through scene ownership, localFocus, a forced interaction, or fresh nearby exploration value.",
    "Delegate scouting goals that explicitly ask the player to talk to one or two fresh NPCs, read signs, inspect item balls, or check unique objects when entering a new area or lacking a route clue.",
    "Treat verified interactable sampling as progress, not a detour, when it produces route clues, items, open-question answers, or a safer next objective.",
    "Treat NPC-given goals and interactive-element clues as first-class planning inputs alongside flow_state; if an NPC tells me to do something or go somewhere, turn that into a concrete next goal when it is actionable.",
    STORY_GATE_RECONCILIATION_GUIDANCE,
    POST_MYSTERY_EGG_MOM_GUIDANCE,
    "Prefer immediate goals like 'reach the nearest verified exit', 'complete the forced interaction', 'talk to the fresh NPC by the route', 'inspect the nearby sign/item/object for a clue', and 'collect the current progression reward' over vague exploration goals.",
    "Use working memory to keep canonical run state up to date: runSummary, discoveries, blockers, routeNotes, partyNotes, npcGoals, interactableNotes, and openQuestions.",
    "Keep working memory notes for verified sign/NPC/interactable clues: what was learned, who or what said it, what action it recommends next, and whether it belongs in discoveries, routeNotes, npcGoals, interactableNotes, blockers, or openQuestions.",
    "Carry forward the last verified route facts and map transitions between batches so the player continues the run instead of re-solving the same room.",
    "Only store durable facts or hypotheses that survived live verification.",
    "When progress is made, update the next immediate goal.",
    "If blocked, say exactly why and what the next safest recovery goal is.",
    "Let the player infer the correct immediate action from the live state instead of micromanaging button sequences.",
    "A batch is not valid unless you delegated real gameplay to the player agent and incorporated the player's live result.",
    CORE_GAMEPLAY_GUIDANCE,
  ].join(" ");
}

export function buildTaskmasterPrompt(
  input: RunnerInput,
  status: Status,
  recentEvents?: string,
): string {
  return [
    `Overall goal: ${input.overallGoal}`,
    `Requested immediate goal (verify against live flow): ${input.immediateGoal}`,
    buildLiveObjectiveGuidance(input, status),
    buildStatusGuidance(status),
    `Badges: ${status.badges}`,
    `Party count: ${status.partyCount}`,
    formatRecentEvents(recentEvents),
    "Mandatory next action: delegate to the player agent now. Do not answer this prompt directly before delegation.",
    "Delegation style: state the concrete route objective, the live evidence that supports it, and the expected type of tool action.",
    "If the goal includes external advice, treat it as high-priority routing guidance after verifying it against live state.",
    "If the goal includes external intervention, manual play already changed the live game; continue from the updated live state instead of replaying those inputs.",
    "If live party/flow evidence shows the immediate goal text is stale, delegate the current flow goal instead of repeating the stale objective.",
    POST_MYSTERY_EGG_MOM_GUIDANCE,
    "Tell the player to nickname newly received or caught Pokemon when the game offers the prompt.",
    "Delegation target: ask the player agent to make bounded live MCP progress toward the live-corrected immediate goal and report exact actions/evidence.",
    "After delegation completes, use the player's live result plus recent state to return the checkpoint.",
    "Return a concise checkpoint with evidence from the live state.",
  ].join("\n");
}

export function buildPlayerDelegationPrompt(
  input: RunnerInput,
  status: Status,
  recentEvents?: string,
): string {
  const localModel = /^ollama\//i.test(input.playerModel) || /^ollama\//i.test(input.taskmasterModel);
  if (localModel) {
    return [
      `Goal: ${truncateForPrompt(input.immediateGoal, 500)}`,
      buildStatusGuidance(status),
      formatCompactRecentEvents(recentEvents),
      "Use live MCP evidence. Take a short valid Game Boy action sequence now.",
      "Before each action, give a concise operational reason.",
      "If battle/menu/dialogue is active, solve that interface first; do not use overworld movement until it closes.",
      "If movement is available, follow live localFocus, openDirections, hotspots, signs, NPCs, item balls, warps, or flow_state toward progress.",
      "If no action is obvious, observe/map_info once, then choose. YOU MUST MAKE A CHOICE.",
      `Use at most ${Math.max(2, Math.min(input.playerMaxSteps, 6))} MCP actions.`,
    ].join("\n");
  }

  return [
    `Requested immediate goal (verify against live flow): ${input.immediateGoal}`,
    buildLiveObjectiveGuidance(input, status),
    buildStatusGuidance(status),
    formatRecentEvents(recentEvents),
    "Execute the next useful gameplay action quickly and correctly. Give received or caught Pokemon a short nickname when a nickname prompt appears unless it blocks urgent progress.",
    "Convert the objective into valid movement, facing, button presses, menu choices, or battle inputs.",
    "If external advice appears, treat it as high-priority routing guidance after verifying it against live state.",
    "If external intervention appears, manual play already changed the live state; observe the current state and continue instead of replaying listed inputs.",
    "Visible reasons must explain live evidence and intended effect with no fictional framing.",
    "Result summaries must be concise, tool-grounded, and free of non-operational flavor.",
    "Follow the KrabbyClaw play loop: status, observe if needed, then take a short verified action sequence before checking status again.",
    `Use up to ${Math.max(2, Math.min(input.playerMaxSteps, 6))} direct MCP actions when the route is clear.`,
    "Drive hard toward the immediate objective. If the target is obvious, do not waste the batch on one-tile hesitation.",
    "Preserve route continuity from the last verified progress. If recent events already advanced toward a target, continue from that route instead of re-exploring.",
    "When localFocus, interactionSetup, or interactionLane identifies a safe target, follow that lane directly.",
    "For visible I tiles, route to the adjacent approach tile, face the I tile, and press A. Never treat the I tile itself as a walkable destination.",
    "If an interactable I-tile objective is directly ahead, press A now instead of moving.",
    "If the only nearby target is an ambient utility/sign/landmark and there is no stronger focus, do not path into it blindly; get map_info and choose either the nearest verified route target or one fresh reachable interactable that can provide a clue.",
    "If you are standing on an ambient utility approach tile but live hotspot context also shows a warp, objective, or required NPC, prioritize the higher-authority route unless the ambient target is fresh, close, and likely to answer the current route question.",
    "If a fresh reachable NPC/sign/object is locally focused and the route is not already forced, interact with it before wandering or leaving; report what was learned.",
    "When an NPC/sign/object gives a goal, request, warning, or item hint, write it down in the result as a concrete note and let it shape the next immediate goal alongside flow_state.",
    POST_MYSTERY_EGG_MOM_GUIDANCE,
    "If the next action is unclear, look around at nearby signs, NPCs, item balls, and unique objects for action clues, interact with the most relevant one when live state supports it, and report the clue as a compact to-do note.",
    "Spend at most one or two exploratory interactions per batch unless they reveal a blocker, item, or objective; then return to route progress.",
    "If forced NPC exposition just ended and visible objective hotspots are now present, stop following the NPC focus and pivot to the nearest objective hotspot immediately.",
    "When localMovement provides openDirections or blockedDirections, treat that as hard local navigation evidence.",
    "Do not choose a direction listed in blockedDirections unless newer live evidence clearly overrides it.",
    "If only one direction is open locally, take it instead of guessing from facing or map intuition.",
    "If the current room clearly has an exit hotspot or stairs, commit to that objective first.",
    "If observe or hotspot text provides a relative target like '5N 6E', use that as a route clue and keep following it while the live state remains consistent.",
    "If no route has been verified yet, take a short probe along a locally open direction that reduces the route to the current hotspot before committing to a long move sequence.",
    "If you just entered a new map and a forced interaction interrupts movement, resolve it cleanly and then resume pathing toward the nearest verified progression target.",
    "If dialogue opens a choice or a menu, stop autopressing and use the live state to decide whether to confirm, move the cursor, or back out.",
    "If textAdvancePending is true without a choice/menu, advance the textbox with a deliberate A press and then re-check status.",
    "When a choice/menu is open, use button presses for cursor movement; do not spend the batch on overworld movement tools until the menu closes.",
    BUTTON_PROMPT_GUIDANCE,
    "If battle is active, stop treating blocked movement as navigation evidence. Pick a battle plan from the visible battle menu: run from low-risk wild blockers when travel is the priority, or fight with sensible damaging moves when escape is unavailable, risky, or already failed.",
    "In battle, B is not a reliable escape from the fight itself; use the RUN command or win the battle.",
    "If FIGHT is already highlighted and the plan is to clear the blocker by battling, confirm FIGHT and choose a move instead of canceling.",
    "If the dialogue is forced and non-branching, confirm deliberately, then re-check whether control returned before taking another action.",
    "If a confirmation or yes/no prompt already highlights the progress-preserving default choice, confirm it once and then reassess from fresh state instead of spending extra presses on the transition itself.",
    "If a prompt turns into a yes/no or simple tutorial choice, prefer the option that continues progress and avoids optional explanation, then confirm deliberately.",
    "If a press returns reason=menu or changes mode to menu, reassess before the next input and re-read status before acting.",
    "If a confirm press returns reason=busy while a prompt is changing, assume the transition is in flight and re-read status or observe before repeating the same confirm.",
    "If direct story progress is blocked, pick the nearest honest exploration or training action that can unlock progress.",
    "If the current objective is only a completed proof, do not repeat that proof; leave the proof location and route toward the follow-up objective established by live evidence.",
    "Do not speculate about unseen geometry when localMovement, interactionSetup, or interactionLane already give safer guidance.",
    "Use the newest live state and recent outcomes to decide whether to continue pathing, interact, confirm, back out, or reassess.",
    "Use concrete MCP tool actions and report the concrete result.",
  ].join("\n");
}

export function buildTaskmasterSummaryPrompt(
  input: RunnerInput,
  beforeStatus: Status,
  afterStatus: Status,
  delegationEvidence: string[],
  recentEvents: string,
  observationText?: string,
): string {
  const compactDelegationEvidence = delegationEvidence
    .slice(-MAX_DELEGATION_ITEMS)
    .map(item => `- ${truncateForPrompt(item, 400)}`)
    .join("\n");

  return [
    `Overall goal: ${input.overallGoal}`,
    `Requested immediate goal (verify against live flow): ${input.immediateGoal}`,
    buildLiveObjectiveGuidance(input, beforeStatus),
    `Before: ${beforeStatus.map} ${formatCoords(beforeStatus.coords)} flow=${beforeStatus.flowNextGoal} badges=${beforeStatus.badges} party=${beforeStatus.partyCount}`,
    `After: ${afterStatus.map} ${formatCoords(afterStatus.coords)} flow=${afterStatus.flowNextGoal} badges=${afterStatus.badges} party=${afterStatus.partyCount}`,
    `Delegation evidence:\n${compactDelegationEvidence || "- None recorded."}`,
    `Recent events:\n${truncateForPrompt(recentEvents)}`,
    observationText ? `Observation:\n${truncateForPrompt(observationText)}` : "Observation:\nNone collected.",
    "Before returning, update working memory with any verified NPC goals, interactable clues, item hints, route notes, blockers, or open questions learned this batch.",
    "Summarize objectively whether the immediate goal is done, still in progress, or blocked.",
    "Set shouldContinue true only if the same immediate goal should be resumed next batch.",
    "Set nextImmediateGoal to the exact next tactical objective.",
    "If an NPC or interactive element gave an actionable goal, it may become nextImmediateGoal even when flow_state is broader.",
    "List evidence grounded only in the delegation evidence, recent events, observation text, and before/after state transition.",
  ].join("\n");
}
