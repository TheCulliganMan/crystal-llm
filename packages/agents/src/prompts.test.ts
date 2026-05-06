import {
  buildPlayerDelegationPrompt,
  buildPlayerInstructions,
  buildTaskmasterInstructions,
  buildTaskmasterPrompt,
  buildTaskmasterSummaryPrompt,
} from "./prompts";

const input = {
  session: {
    baseUrl: "http://127.0.0.1:3000",
    sessionId: "run-123",
  },
  overallGoal: "Beat Pokemon Crystal.",
  immediateGoal: "Leave the bedroom and go downstairs.",
  taskmasterModel: "openai/gpt-5.4",
  playerModel: "openai/gpt-5.4-mini",
  supervisorMaxSteps: 8,
  playerMaxSteps: 5,
  autoSuspend: true,
  includeObservationCheckpoint: true,
} as const;

const status = {
  mode: "overworld",
  map: "PlayersHouse2F",
  mapId: "24:7",
  coords: [3, 3],
  facing: "down",
  badges: 0,
  canMove: true,
  partyCount: 0,
  flowSummary: "Next goal: Starter + Pokédex",
  flowNextGoal: "Starter + Pokédex",
  flowCompletionTarget: "Beat Mt. Silver",
  localFocus: {
    source: "status",
    target: {
      kind: "hotspot",
      coords: [3, 11],
      label: "Stairs",
      token: "D>",
      hotspotType: "warp",
    },
    recommendedApproach: {
      coords: [3, 11],
      facing: "down",
      setupFrom: [3, 9],
    },
  },
  localMovement: {
    openDirections: [{ direction: "down", tile: "." }],
    blockedDirections: [{ direction: "right", tile: "#" }],
  },
} as const;

describe("prompt builders", () => {
  it("keeps the player instructions honest and MCP-focused", () => {
    const instructions = buildPlayerInstructions();

    expect(instructions).toContain("Never use execute_macro");
    expect(instructions).toContain("controlling Pokemon Crystal through MCP tools");
    expect(instructions).toContain("beating the game quickly and correctly");
    expect(instructions).toContain("fast, reliable progression");
    expect(instructions).toContain("Visible reasons and reports must be concise operational explanations");
    expect(instructions).toContain("external advice");
    expect(instructions).toContain("external intervention");
    expect(instructions).toContain("Never invent game state");
    expect(instructions).toContain("prefer giving that Pokemon a short nickname");
    expect(instructions).toContain("MCP gameplay tools directly");
    expect(instructions).toContain("Flow_state is the sequential backbone for beating the game");
    expect(instructions).toContain("everything encountered on the honest route toward that flow goal as important route evidence");
    expect(instructions).toContain("required NPCs, signs, item balls, forced prompts, battles, doors, warps, blockers, and local clues");
    expect(instructions).toContain("required visible reason field");
    expect(instructions).toContain("localFocus");
    expect(instructions).toContain("map_info");
    expect(instructions).toContain("2-6 honest inputs");
    expect(instructions).toContain("productive route or start honest training progress");
    expect(instructions).toContain("Infer the next best valid input from the live status");
    expect(instructions).toContain("reassess from the newest live evidence");
    expect(instructions).toContain("If localMovement says a direction is blocked");
    expect(instructions).toContain("If localMovement shows only one safe direction");
    expect(instructions).toContain("Visible I tiles are item balls or starter poke balls");
    expect(instructions).toContain("face the I tile, then press A");
    expect(instructions).toContain("do not try to walk onto the I tile");
    expect(instructions).toContain("utility/sign/landmark/trigger targets as ambient");
    expect(instructions).toContain("immediately use map_info and choose either the nearest verified route target");
    expect(instructions).toContain("unless the ambient target is fresh, close, and likely to answer the current route question");
    expect(instructions).toContain("Priority 1: advance the current story flow");
    expect(instructions).toContain("next major progression unlock");
    expect(instructions).toContain("fast and robust");
    expect(instructions).toContain("Listen to NPCs and interactive elements as goal sources");
    expect(instructions).toContain("Take notes from NPCs and interactive elements");
    expect(instructions).toContain("nearby fresh NPCs, signs, item balls, and unique objects as worthwhile exploration");
    expect(instructions).toContain("Prefer high-information interactables");
    expect(instructions).toContain("Avoid repeating the same inert target");
    expect(instructions).toContain("NPC goal rule");
    expect(instructions).toContain("Note-taking rule");
    expect(instructions).toContain("nearest verified route transition");
    expect(instructions).toContain("compact internal memory of the last verified route facts");
    expect(instructions).toContain("signs, NPCs, item balls, and unique objects for action clues");
    expect(instructions).toContain("write down the resulting to-do as a compact route note");
    expect(instructions).toContain("hotspot deltas like '5N 6E'");
    expect(instructions).toContain("avoid a long blind burst into an unconfirmed direction");
    expect(instructions).toContain("yes/no or simple tutorial choice");
    expect(instructions).toContain("If textAdvancePending is true");
    expect(instructions).toContain("call status again before deciding on another action");
    expect(instructions).toContain("use button presses for cursor movement and confirmation");
    expect(instructions).toContain("A means forward");
    expect(instructions).toContain("B means back/exit");
    expect(instructions).toContain("use menu cursor movement otherwise");
    expect(instructions).toContain("Post-Mystery-Egg rule");
    expect(instructions).toContain("go to Player's House 1F, talk to Mom");
    expect(instructions).toContain("decline or cancel Mom saving money");
    expect(instructions).toContain("battle menu is the current problem to solve");
    expect(instructions).toContain("do not try overworld movement or repeatedly press B");
    expect(instructions).toContain("visible objective hotspots");
    expect(instructions).not.toContain("PlayersHouse2F");
    expect(instructions).not.toContain("Elm's Lab");
    expect(instructions).not.toContain("in-world");
    expect(instructions).not.toContain("trainer in motion");
    expect(instructions).not.toContain("trainer affirmations");
    expect(instructions).not.toContain("Professor Culligan");
  });

  it("makes the planner delegate actual play to the player agent", () => {
    const instructions = buildTaskmasterInstructions();

    expect(instructions).toContain("gameplay planner");
    expect(instructions).toContain("fast and correct clear");
    expect(instructions).toContain("objective, operational language only");
    expect(instructions).toContain("external advice");
    expect(instructions).toContain("external intervention");
    expect(instructions).toContain("Delegate concrete gameplay execution to the player agent");
    expect(instructions).toContain("must not invent game state");
    expect(instructions).toContain("Recommend nicknaming every Pokemon received or caught");
    expect(instructions).toContain("Hard requirement: every gameplay batch must delegate");
    expect(instructions).toContain("first meaningful action in each gameplay batch is to call the player agent");
    expect(instructions).toContain("Never return a checkpoint based only on your own reasoning");
    expect(instructions).toContain("If you are unsure what to do, delegate a scouting objective");
    expect(instructions).toContain("Delegate all concrete gameplay execution");
    expect(instructions).toContain("live flow_state and party evidence");
    expect(instructions).toContain("Flow_state is the sequential backbone for beating the game");
    expect(instructions).toContain("everything encountered on the honest route toward that flow goal as important route evidence");
    expect(instructions).toContain("player agent");
    expect(instructions).toContain("incorporated the player's live result");
    expect(instructions).toContain("instead of micromanaging button sequences");
    expect(instructions).toContain("Base nudges on structured MCP evidence");
    expect(instructions).toContain("Treat lone utility/sign/landmark/trigger targets as ambient");
    expect(instructions).toContain("fresh nearby exploration value");
    expect(instructions).toContain("talk to one or two fresh NPCs");
    expect(instructions).toContain("verified interactable sampling as progress");
    expect(instructions).toContain("NPC-given goals and interactive-element clues as first-class planning inputs");
    expect(instructions).toContain("Post-Mystery-Egg rule");
    expect(instructions).toContain("talk to Mom");
    expect(instructions).toContain("money-saving/bank prompt");
    expect(instructions).toContain("forced NPC exposition ends");
    expect(instructions).toContain("Priority 1: advance the current story flow");
    expect(instructions).toContain("Drive fast, competent progression");
    expect(instructions).toContain("npcGoals");
    expect(instructions).toContain("interactableNotes");
    expect(instructions).toContain("working memory notes for verified sign/NPC/interactable clues");
    expect(instructions).toContain("what action it recommends next");
    expect(instructions).toContain("Carry forward the last verified route facts");
    expect(instructions).not.toContain("Elm's Lab");
    expect(instructions).not.toContain("trainer's self");
    expect(instructions).not.toContain("in-world");
    expect(instructions).not.toContain("affirmations");
    expect(instructions).not.toContain("Professor Culligan");
  });

  it("builds stable taskmaster prompts with current state", () => {
    const recentEvents = "move:down:1 ok\nmove:right:1 ok";

    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain(input.immediateGoal);
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("Live objective authority");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("any newly verified NPC/interactable goal");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("Flow_state is the sequential backbone for beating the game");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("can all be the actual next step");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("Mandatory next action: delegate to the player agent now");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("state the concrete route objective");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("external advice");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("external intervention");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("nickname newly received or caught Pokemon");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("Post-Mystery-Egg rule");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("resolve her money-saving/bank prompt deliberately");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("Do not answer this prompt directly before delegation");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("After delegation completes");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("Local focus");
    expect(buildTaskmasterPrompt(input, status, recentEvents)).toContain("Recent live actions/events");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Open adjacent movement");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Live objective authority");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Flow_state is the sequential backbone for beating the game");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("required NPCs, signs, item balls, forced prompts, battles, doors, warps, blockers, and local clues");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Execute the next useful gameplay action quickly and correctly");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Convert the objective into valid movement");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("external advice");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("external intervention");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Visible reasons must explain live evidence");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Result summaries must be concise");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Give received or caught Pokemon a short nickname");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Recent live actions/events");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Use up to 5 direct MCP actions");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Use the newest live state and recent outcomes");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Do not choose a direction listed in blockedDirections");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("For visible I tiles");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("press A now instead of moving");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("If the current room clearly has an exit hotspot or stairs");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("ambient utility/sign/landmark");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("standing on an ambient utility approach tile");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("fresh reachable NPC/sign/object");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("write it down in the result as a concrete note");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("nearby signs, NPCs, item balls, and unique objects for action clues");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("compact to-do note");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Preserve route continuity from the last verified progress");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("relative target like '5N 6E'");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("short probe along a locally open direction");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("prefer the option that continues progress and avoids optional explanation");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("If textAdvancePending is true without a choice/menu");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("use button presses for cursor movement");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("A means forward");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("B means back/exit");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("use menu cursor movement otherwise");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("Post-Mystery-Egg rule");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("do not route straight toward Violet");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("If battle is active");
    expect(buildPlayerDelegationPrompt(input, status, recentEvents)).toContain("forced NPC exposition just ended");
    expect(
      buildTaskmasterSummaryPrompt(
        input,
        status,
        status,
        ["Delegated one move action."],
        "recent_events: move changed state",
        "OVERWORLD\nPos: (3,3)",
      ),
    ).toContain("Delegation evidence:");
    expect(
      buildTaskmasterSummaryPrompt(
        input,
        status,
        status,
        ["NPC said the route ahead needs training."],
        "recent_events: talked to NPC",
        "OVERWORLD\nPos: (3,3)",
      ),
    ).toContain("verified NPC goals");
    expect(
      buildTaskmasterSummaryPrompt(
        input,
        status,
        status,
        ["NPC said the route ahead needs training."],
        "recent_events: talked to NPC",
        "OVERWORLD\nPos: (3,3)",
      ),
    ).toContain("may become nextImmediateGoal");
  });

  it("truncates oversized summary inputs to keep the next batch compact", () => {
    const hugeText = "x".repeat(1500);
    const prompt = buildTaskmasterSummaryPrompt(
      input,
      status,
      status,
      [hugeText, hugeText, hugeText, hugeText, hugeText],
      hugeText,
      hugeText,
    );

    expect(prompt).toContain("...[truncated]");
    expect(prompt).not.toContain(`- ${"x".repeat(800)}`);
  });

  it("surfaces dialogue and menu state without hardcoding the next button sequence", () => {
    const promptStatus = {
      ...status,
      canMove: false,
      inDialog: true,
      promptPending: true,
      textBoxOpen: true,
      inMenu: true,
      scriptBusy: true,
      movementLocked: true,
      blockedReason: "prompt",
    } as const;

    const prompt = buildPlayerDelegationPrompt(input, promptStatus, "prompt opened");

    expect(prompt).toContain("Dialogue/prompt is active.");
    expect(prompt).toContain("Menu is active.");
    expect(prompt).toContain("Movement is currently locked/busy.");
    expect(prompt).not.toContain("Use A to advance text");
  });

  it("surfaces textbox continuation separately from prompt menus", () => {
    const prompt = buildPlayerDelegationPrompt(
      input,
      {
        ...status,
        canMove: false,
        inDialog: true,
        textBoxOpen: true,
        textAdvancePending: true,
      },
      "dialogue waiting"
    );

    expect(prompt).toContain("Text advance is pending.");
    expect(prompt).not.toContain("Menu is active.");
  });

  it("corrects stale starter goals from live party and flow evidence", () => {
    const prompt = buildPlayerDelegationPrompt(
      {
        ...input,
        immediateGoal: "Get the starter Pokemon, then continue the journey.",
      },
      {
        ...status,
        map: "Route29",
        partyCount: 1,
        flowSummary: "Next goal: Mr. Pokemon + Mystery Egg",
        flowNextGoal: "Mr. Pokemon + Mystery Egg",
      },
      "flow_state completed starter"
    );

    expect(prompt).toContain("Starter objective correction");
    expect(prompt).toContain("do not keep trying to get a starter");
    expect(prompt).toContain("pivot to the current flow goal");
  });

  it("prioritizes the Mom handoff and bank prompt after the Mystery Egg return", () => {
    const postEggStatus = {
      ...status,
      map: "NewBarkTown",
      mapId: "NEW_BARK_TOWN",
      partyCount: 2,
      flowSummary: "Next goal: Mom + money setup",
      flowNextGoal: "Mom + money setup",
      localFocus: {
        source: "status",
        target: {
          kind: "hotspot",
          label: "Mom",
          token: "M",
          hotspotType: "npc",
          script: "MomScript",
        },
      },
    } as const;

    const prompt = buildPlayerDelegationPrompt(
      {
        ...input,
        immediateGoal: "Return the Mystery Egg to Elm, then leave for Violet.",
      },
      postEggStatus,
      "EVENT_GAVE_MYSTERY_EGG_TO_ELM set; Mom banking not cleared",
    );

    expect(prompt).toContain("Current flow goal: Mom + money setup");
    expect(prompt).toContain("go to Player's House 1F, talk to Mom");
    expect(prompt).toContain("decline or cancel Mom saving money");
    expect(prompt).toContain("do not route straight toward Violet");
  });

  it("frames battle state as a solvable menu instead of a movement blocker", () => {
    const prompt = buildPlayerDelegationPrompt(
      input,
      {
        ...status,
        mode: "battle",
        canMove: false,
        inBattle: true,
        inMenu: true,
        blockedReason: "battle",
        surface: {
          kind: "battle",
          title: "Battle",
          selected: "FIGHT",
          primaryText: "> FIGHT",
        },
      },
      "move:up:1 busy"
    );

    expect(prompt).toContain("Battle is active; resolve the battle interface");
    expect(prompt).toContain("Pick a battle plan from the visible battle menu");
    expect(prompt).toContain("B is not a reliable escape");
    expect(prompt).toContain("confirm FIGHT and choose a move");
  });
});
