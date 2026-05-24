import {
  agentStreamEventsFromTaskmasterBatch,
  buildCodexBatchPrompt,
  buildCodexToolDefinitions,
  buildNoActionAgenticBatch,
  codexCliStreamEventFromJsonLine,
  extractCodexAgentMessages,
  extractJsonObjectFromText,
  getCodexTurnTimeoutMs,
  isCodexModel,
  normalizeCodexButtonArgs,
  normalizeCodexFrameCount,
  normalizeCodexMoveArgs,
  readCodexVisibleActionReason,
  summarizeCodexRouteTarget,
} from "./codex-harness";
import {
  DEFAULT_OPENAI_MODEL,
  normalizeAgentModel,
  resolveMastraModel,
} from "./defaults";
import type { RunnerInput, Status } from "./types";

describe("codex harness helpers", () => {
  it("recognizes codex model refs", () => {
    expect(isCodexModel("codex/gpt-5.4")).toBe(true);
    expect(isCodexModel("openai/gpt-5.4")).toBe(false);
  });

  it("extracts a raw JSON object from plain text", () => {
    expect(extractJsonObjectFromText('{"ok":true}')).toBe('{"ok":true}');
  });

  it("extracts a fenced JSON object from model output", () => {
    expect(extractJsonObjectFromText("```json\n{\"ok\":true}\n```")).toBe('{"ok":true}');
  });

  it("normalizes codex aliases into the codex harness path", () => {
    expect(normalizeAgentModel("codex")).toBe("codex/gpt-5.4");
    expect(normalizeAgentModel("openai-codex")).toBe("codex/gpt-5.4");
    expect(normalizeAgentModel("openai-codex/gpt-5.4")).toBe("codex/gpt-5.4");
    expect(resolveMastraModel("openai/gpt-5.4")).toBe(DEFAULT_OPENAI_MODEL);
    expect(resolveMastraModel("codex/gpt-5.4")).toBe("codex/gpt-5.4");
  });

  it("maps codex exec JSONL text, reasoning, and MCP records into TUI stream events", () => {
    expect(
      codexCliStreamEventFromJsonLine(JSON.stringify({
        type: "item.updated",
        item: { id: "reason-1", type: "reasoning", text: "Need to inspect Elm's Lab." },
      }))
    ).toEqual({
      type: "thinking-delta",
      text: "Need to inspect Elm's Lab.",
      source: "codex",
    });
    expect(
      codexCliStreamEventFromJsonLine(JSON.stringify({
        type: "item.started",
        item: { id: "tool-1", type: "mcp_tool_call", server: "krabbyclaw", server_tool: "observe" },
      }))
    ).toEqual({
      type: "mcp-call",
      name: "krabbyclaw.observe",
      source: "codex",
    });
    expect(
      codexCliStreamEventFromJsonLine(JSON.stringify({
        type: "item.updated",
        item: { id: "msg-1", type: "agentMessage", text: "I can see the starter balls." },
      }))
    ).toEqual({
      type: "text-delta",
      text: "I can see the starter balls.",
      source: "codex",
    });
  });

  it("renders codex checkpoint summaries as visible reasoning and output", () => {
    expect(
      agentStreamEventsFromTaskmasterBatch({
        summary: "Picked Cyndaquil and confirmed the prompt.",
        immediateGoalStatus: "done",
        nextImmediateGoal: "Leave Elm's Lab",
        shouldContinue: true,
        evidence: ["party=1", "dialogue advanced"],
      })
    ).toEqual([
      {
        type: "thinking-delta",
        text: "Decision: Picked Cyndaquil and confirmed the prompt. Evidence: party=1 | dialogue advanced",
        source: "codex",
      },
      {
        type: "text-delta",
        text: "Goal done; next: Leave Elm's Lab; continue: true.",
        source: "codex",
      },
    ]);
  });

  it("emits only the new suffix when codex exec repeats full message snapshots", () => {
    const accumulator = { textByItemId: new Map<string, string>() };

    expect(
      codexCliStreamEventFromJsonLine(
        JSON.stringify({
          type: "item.updated",
          item: { id: "msg-1", type: "agentMessage", text: "Press A" },
        }),
        accumulator
      )?.text
    ).toBe("Press A");
    expect(
      codexCliStreamEventFromJsonLine(
        JSON.stringify({
          type: "item.updated",
          item: { id: "msg-1", type: "agentMessage", text: "Press A, then wait" },
        }),
        accumulator
      )?.text
    ).toBe(", then wait");
  });

  it("clamps move and frame arguments to Game Boy-safe bounds", () => {
    expect(normalizeCodexMoveArgs({ direction: "right", steps: 4 })).toEqual({
      direction: "right",
      steps: 4,
    });
    expect(normalizeCodexMoveArgs({ direction: "right", steps: 9 })).toEqual({
      direction: "right",
      steps: 4,
    });
    expect(normalizeCodexMoveArgs({ direction: "bad" })).toEqual({
      direction: "down",
      steps: 1,
    });
    expect(normalizeCodexFrameCount(200, 20, 60)).toBe(60);
    expect(normalizeCodexFrameCount(undefined, 20, 60)).toBe(20);
    expect(normalizeCodexButtonArgs({ button: "oops" })).toEqual({ button: "A" });
    expect(normalizeCodexButtonArgs({ button: "Down" })).toEqual({ button: "Down" });
    expect(normalizeCodexButtonArgs({ button: " a " })).toEqual({ button: "A" });
    expect(normalizeCodexButtonArgs({ button: "b" })).toEqual({ button: "B" });
    expect(normalizeCodexButtonArgs({ button: "start" })).toEqual({ button: "Start" });
  });

  it("requires a visible reason on every self-managed Codex action tool", () => {
    const tools = buildCodexToolDefinitions({} as never, 4);
    for (const name of ["move", "press", "hold_button"]) {
      const tool = tools.find((entry: { spec: { name: string } }) => entry.spec.name === name);
      expect(tool?.spec.inputSchema).toMatchObject({
        properties: {
          reason: { type: "string" },
        },
        required: expect.arrayContaining(["reason"]),
        additionalProperties: false,
      });
    }
    expect(tools.some((entry: { spec: { name: string } }) => entry.spec.name === "route_render")).toBe(true);
    expect(tools.some((entry: { spec: { name: string } }) => entry.spec.name === "wait")).toBe(false);
  });

  it("extracts concise visible Codex action reasons", () => {
    expect(
      readCodexVisibleActionReason({
        reason: "  I see the ball in front of me, so pressing A should inspect it.  ",
      }),
    ).toBe("I see the ball in front of me, so pressing A should inspect it.");
    expect(readCodexVisibleActionReason({ reason: "" })).toBeNull();
    expect(readCodexVisibleActionReason({})).toBeNull();
  });

  it("uses a sane default codex turn timeout and rejects invalid overrides", () => {
    const previous = process.env.POKECRYSTAL_CODEX_TURN_TIMEOUT_MS;
    delete process.env.POKECRYSTAL_CODEX_TURN_TIMEOUT_MS;
    expect(getCodexTurnTimeoutMs()).toBe(90000);

    process.env.POKECRYSTAL_CODEX_TURN_TIMEOUT_MS = "120000";
    expect(getCodexTurnTimeoutMs()).toBe(120000);

    process.env.POKECRYSTAL_CODEX_TURN_TIMEOUT_MS = "1000";
    expect(() => getCodexTurnTimeoutMs()).toThrow(
      "POKECRYSTAL_CODEX_TURN_TIMEOUT_MS must be an integer >= 5000 when set."
    );

    if (previous === undefined) {
      delete process.env.POKECRYSTAL_CODEX_TURN_TIMEOUT_MS;
    } else {
      process.env.POKECRYSTAL_CODEX_TURN_TIMEOUT_MS = previous;
    }
  });

  it("builds a concise self-managed codex batch prompt with live state", () => {
    const prompt = buildCodexBatchPrompt(
      {
        session: {
          sessionId: "session-1",
          baseUrl: "http://127.0.0.1:3000",
        },
        immediateGoal: "Advance the current story.",
        overallGoal: "Beat Pokemon Crystal.",
        taskmasterModel: "codex/gpt-5.4",
        playerModel: "codex/gpt-5.4",
        supervisorMaxSteps: 4,
        playerMaxSteps: 4,
        autoSuspend: true,
        includeObservationCheckpoint: true,
      },
      {
        mode: "overworld",
        map: "PlayersHouse2F",
        mapId: "PLAYERS_HOUSE_2F",
        coords: [3, 3],
        facing: "down",
        badges: 0,
        canMove: true,
        partyCount: 0,
        flowSummary: "Next goal: Starter + Pokedex",
        flowNextGoal: "Starter + Pokedex",
        flowCompletionTarget: "Beat Mt. Silver",
        localFocus: {
          source: "status",
          target: {
            kind: "hotspot",
            label: "Warp",
            token: "D",
            coords: [7, 0],
            hotspotType: "warp",
          },
          recommendedApproach: {
            coords: [7, 1],
            facing: "up",
            setupFrom: [5, 1],
          },
        },
        localMovement: {
          openDirections: [{ direction: "down", tile: "." }],
          blockedDirections: [{ direction: "right", tile: "#" }],
        },
      },
      "move:right:4 blocked",
      '{"map":"PlayersHouse2F","player":{"coords":{"x":3,"y":3}},"hotspots":[{"type":"warp","label":"Warp: Players House1 F","coords":{"x":15,"y":1}}]}',
      "OVERWORLD\nHOTSPOTS\nD Warp: Players House1 F (1N 6E)",
    );

    expect(prompt).toContain("Self-manage one bounded Pokemon Crystal gameplay batch");
    expect(prompt).toContain("Overall goal: Beat Pokemon Crystal.");
    expect(prompt).toContain("Requested immediate goal (verify against live flow): Advance the current story.");
    expect(prompt).toContain("Live status before acting:");
    expect(prompt).toContain("Live map_info before acting:");
    expect(prompt).toContain("Live observe before acting:");
    expect(prompt).toContain("Warp: Players House1 F");
    expect(prompt).toContain('"map":"PlayersHouse2F"');
    expect(prompt).toContain("if status and observe disagree");
    expect(prompt).toContain("objective operational language only");
    expect(prompt).toContain("fictional framing");
    expect(prompt).toContain("concise operational reason");
    expect(prompt).toContain("prefer giving that Pokemon a short nickname");
    expect(prompt).toContain("Live objective authority");
    expect(prompt).toContain("If live party/flow evidence shows the immediate goal text is stale");
    expect(prompt).toContain("Flow_state is the sequential backbone for beating the game");
    expect(prompt).toContain("everything encountered on the honest route toward that flow goal as important route evidence");
    expect(prompt).toContain("may be the actual next step");
    expect(prompt).toContain("newly verified NPC/interactable goals as actionable objectives");
    expect(prompt).toContain("NPC requests, sign clues, item hints, unique objects, forced prompts, battles, doors, warps, blockers, and local clues");
    expect(prompt).toContain("Post-Mystery-Egg route");
    expect(prompt).toContain("talk to Mom in Player's House 1F");
    expect(prompt).toContain("Decline or cancel Mom saving money");
    expect(prompt).toContain("nearby signs, NPCs, item balls, and unique objects for action clues");
    expect(prompt).toContain("sample one fresh reachable NPC/sign/object");
    expect(prompt).toContain("compact note about what to do next");
    expect(prompt).toContain("Every action tool call");
    expect(prompt).toContain("visible reason");
    expect(prompt).toContain("Return exactly one JSON object");
  });

  it("adds decisive battle and stale-starter guidance to self-managed codex prompts", () => {
    const prompt = buildCodexBatchPrompt(
      {
        session: {
          sessionId: "session-1",
          baseUrl: "http://127.0.0.1:3000",
        },
        immediateGoal: "Get the starter Pokemon.",
        overallGoal: "Beat Pokemon Crystal.",
        taskmasterModel: "codex/gpt-5.4",
        playerModel: "codex/gpt-5.4",
        supervisorMaxSteps: 4,
        playerMaxSteps: 4,
        autoSuspend: true,
        includeObservationCheckpoint: true,
      },
      {
        mode: "battle",
        surface: {
          kind: "battle",
          title: "Battle",
          selected: "FIGHT",
          primaryText: "> FIGHT",
        },
        map: "Route29",
        mapId: "ROUTE_29",
        coords: [95, 29],
        facing: "up",
        badges: 0,
        canMove: false,
        inBattle: true,
        inMenu: true,
        blockedReason: "battle",
        partyCount: 1,
        flowSummary: "Next goal: Mr. Pokemon + Mystery Egg",
        flowNextGoal: "Mr. Pokemon + Mystery Egg",
        flowCompletionTarget: "Beat Mt. Silver",
      },
      "move:up:1 busy",
      undefined,
      "BATTLE\nMENU\n> FIGHT\nRUN"
    );

    expect(prompt).toContain("Starter objective correction");
    expect(prompt).toContain("Battle is active now");
    expect(prompt).toContain("do not try overworld movement or repeatedly press B");
    expect(prompt).toContain("use the RUN command or win the battle");
  });

  it("turns stalled model batches into no-action agentic checkpoints", () => {
    const input: RunnerInput = {
      session: {
        sessionId: "session-1",
        baseUrl: "http://127.0.0.1:3000",
      },
      immediateGoal: "Reach Cherrygrove.",
      overallGoal: "Beat Pokemon Crystal.",
      taskmasterModel: "ollama/gemma4:26b",
      playerModel: "ollama/gemma4:26b",
      supervisorMaxSteps: 4,
      playerMaxSteps: 1,
      autoSuspend: true,
      includeObservationCheckpoint: true,
    };
    const beforeStatus: Status = {
      mode: "overworld",
      map: "Route29",
      mapId: "ROUTE_29",
      coords: [89, 31],
      facing: "right",
      badges: 0,
      canMove: true,
      partyCount: 1,
      flowSummary: "Next goal: Mr. Pokemon + Mystery Egg",
      flowNextGoal: "Mr. Pokemon + Mystery Egg",
      flowCompletionTarget: "Beat Mt. Silver",
    };

    const batch = buildNoActionAgenticBatch({
      input,
      beforeStatus,
      afterStatus: beforeStatus,
      recentEvents: JSON.stringify({ total: 20, summary: "press:a:1 no_change" }),
      reason: "Direct player agent produced no real gameplay progress.",
    });

    expect(batch.summary).toContain("No non-agentic gameplay action was executed");
    expect(batch.summary).toContain("YOU MUST MAKE A CHOICE");
    expect(batch.nextImmediateGoal).toContain("YOU MUST MAKE A CHOICE using live MCP evidence");
    expect(batch.immediateGoalStatus).toBe("in_progress");
  });

  it("summarizes the highest-priority live route target from map_info", () => {
    const summary = summarizeCodexRouteTarget(
      JSON.stringify({
        map: "PlayersHouse2F",
        player: { coords: { x: 3, y: 3 }, facing: "down" },
        hotspots: [
          { type: "utility", label: "PC", coords: { x: 5, y: 3 }, visible: true, interactable: true },
          { type: "warp", label: "Warp: Players House1 F", coords: { x: 15, y: 1 }, visible: true, interactable: true },
        ],
      }),
    );

    expect(summary).toContain("Warp: Players House1 F");
    expect(summary).toContain("2N 12E");
    expect(summary).toContain("ambient utility/sign hotspots");
    expect(summary).toContain("sample a fresh nearby NPC/sign/object");
  });

  it("prefers the final-answer agent message over commentary-only turn items", () => {
    const extracted = extractCodexAgentMessages([
      {
        type: "agentMessage",
        id: "commentary-1",
        text: "Still working through the live batch.",
        phase: "commentary",
      },
      {
        type: "agentMessage",
        id: "final-1",
        text: '{"summary":"Progress made","immediateGoalStatus":"in_progress","nextImmediateGoal":"Continue","shouldContinue":true,"evidence":["status ok","action advanced"]}',
        phase: "final_answer",
      },
    ]);

    expect(extracted).toEqual({
      finalText:
        '{"summary":"Progress made","immediateGoalStatus":"in_progress","nextImmediateGoal":"Continue","shouldContinue":true,"evidence":["status ok","action advanced"]}',
      error: null,
    });
  });
});
