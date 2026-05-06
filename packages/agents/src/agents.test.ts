import {
  buildAnthropicActionRequest,
  buildAzureOpenAIActionRequest,
  buildGoogleActionRequest,
  buildLocalOllamaActionRequest,
  buildOpenAIDirectActionRequest,
  consumeAgentStream,
  didRecentEventsAdvance,
  hasDirectPlayerBatchProgress,
  isLocalActionRejectedByRecentFailure,
  isPlayerDelegationPrimitive,
  sanitizeLocalPromptMapInfo,
  sanitizeLocalPromptObservation,
  sanitizeLocalPromptStatus,
  shouldContinueTaskmasterIteration,
  shouldUseDirectPlayerBatch,
} from "./agents";
import type { Status } from "./types";

describe("taskmaster iteration control", () => {
  it("keeps iterating until the player has been delegated", () => {
    expect(
      shouldContinueTaskmasterIteration({
        delegatedToPlayer: false,
        iteration: 1,
        supervisorMaxSteps: 5,
      }),
    ).toEqual({
      continue: true,
      feedback:
        "You have not yet delegated real gameplay to the player agent. Delegate now and use that result before concluding the batch.",
    });
  });

  it("stops iterating once live gameplay has been delegated", () => {
    expect(
      shouldContinueTaskmasterIteration({
        delegatedToPlayer: true,
        iteration: 1,
        supervisorMaxSteps: 5,
      }),
    ).toEqual({
      continue: false,
    });
  });

  it("recognizes Mastra's streamed subagent primitive id", () => {
    expect(isPlayerDelegationPrimitive("player")).toBe(true);
    expect(isPlayerDelegationPrimitive("agent-player")).toBe(true);
    expect(isPlayerDelegationPrimitive("taskmaster")).toBe(false);
  });

  it("routes local ollama models through direct player batches", () => {
    expect(shouldUseDirectPlayerBatch({
      taskmasterModel: "ollama/gemma-4-E4B-it-Q4_K_M.gguf",
      playerModel: "ollama/gemma-4-E4B-it-Q4_K_M.gguf",
    })).toBe(true);
    expect(shouldUseDirectPlayerBatch({
      taskmasterModel: "openai/gpt-5.4",
      playerModel: "openai/gpt-5.4",
    })).toBe(false);
    expect(shouldUseDirectPlayerBatch({
      taskmasterModel: "azure-openai/gpt-5.4-mini",
      playerModel: "azure-openai/gpt-5.4-mini",
    })).toBe(true);
    expect(shouldUseDirectPlayerBatch({
      taskmasterModel: "openai-direct/gpt-5.4-mini",
      playerModel: "openai-direct/gpt-5.4-mini",
    })).toBe(true);
    expect(shouldUseDirectPlayerBatch({
      taskmasterModel: "anthropic/claude-sonnet-4-5",
      playerModel: "anthropic/claude-sonnet-4-5",
    })).toBe(true);
    expect(shouldUseDirectPlayerBatch({
      taskmasterModel: "google/gemini-2.5-flash",
      playerModel: "google/gemini-2.5-flash",
    })).toBe(true);
    expect(shouldUseDirectPlayerBatch({
      taskmasterModel: "gemini/gemini-2.5-flash",
      playerModel: "gemini/gemini-2.5-flash",
    })).toBe(true);
  });

  it("treats recent event growth as direct-player batch progress", () => {
    expect(didRecentEventsAdvance(
      JSON.stringify({ total: 51, summary: "dialogue pending" }),
      JSON.stringify({
        total: 52,
        summary: "dialogue advanced @ ElmsLab",
        events: [{ summary: "dialogue advanced", changed: true }],
      }),
    )).toBe(true);
    expect(didRecentEventsAdvance(
      JSON.stringify({ total: 52, summary: "dialogue advanced @ ElmsLab" }),
      JSON.stringify({ total: 52, summary: "dialogue advanced @ ElmsLab" }),
    )).toBe(false);
  });

  it("does not treat no-change recent event growth as progress", () => {
    expect(didRecentEventsAdvance(
      JSON.stringify({
        total: 350,
        summary: "mode:battle->overworld @ Route29 85,31",
        events: [{ summary: "mode:battle->overworld", changed: true }],
      }),
      JSON.stringify({
        total: 351,
        summary: "press:a:1 no_change @ Route29 85,31",
        events: [
          { summary: "mode:battle->overworld", changed: true },
          { summary: "press:a:1 no_change", action: "press:a:1", changed: false, reason: "no_change" },
        ],
      }),
    )).toBe(false);
  });

  it("does not fail a direct player batch that only advanced dialogue events", () => {
    const status: Status = {
      mode: "overworld",
      map: "ElmsLab",
      location: "ElmsLab",
      mapId: "24:4",
      coords: [9, 9],
      facing: "up",
      badges: 0,
      inMenu: false,
      inDialog: true,
      promptPending: true,
      movementLocked: true,
      canMove: false,
      blockedReason: "dialogue",
      partyCount: 0,
      flowSummary: "Next goal: Starter",
      flowNextGoal: "Starter",
      flowCompletionTarget: "Beat Mt. Silver",
    };

    expect(hasDirectPlayerBatchProgress({
      beforeStatus: status,
      afterStatus: status,
      recentEventsBefore: JSON.stringify({ total: 51, summary: "dialogue pending @ ElmsLab" }),
      recentEventsAfter: JSON.stringify({
        total: 52,
        summary: "dialogue advanced @ ElmsLab",
        events: [{ summary: "dialogue advanced", changed: true }],
      }),
      playerText: "",
    })).toBe(true);
  });

  it("does not treat text-only player output as gameplay progress", () => {
    const status: Status = {
      mode: "overworld",
      map: "Route29",
      mapId: "24:3",
      coords: [85, 31],
      facing: "right",
      badges: 0,
      canMove: true,
      partyCount: 1,
      flowSummary: "Next goal: Mr. Pokémon + Mystery Egg",
      flowNextGoal: "Mr. Pokémon + Mystery Egg",
      flowCompletionTarget: "Beat Mt. Silver",
    };

    expect(hasDirectPlayerBatchProgress({
      beforeStatus: status,
      afterStatus: status,
      recentEventsBefore: JSON.stringify({ total: 350, summary: "press:a:1 no_change @ Route29 85,31" }),
      recentEventsAfter: JSON.stringify({
        total: 351,
        summary: "press:a:1 no_change @ Route29 85,31",
        events: [{ summary: "press:a:1 no_change", action: "press:a:1", changed: false, reason: "no_change" }],
      }),
      playerText: "I pressed A and will continue.",
    })).toBe(false);
  });

  it("builds strict local Ollama JSON-schema requests with map context", () => {
    const status: Status = {
      mode: "overworld",
      map: "SomeMap",
      mapId: "24:7",
      coords: [3, 3],
      facing: "up",
      badges: 0,
      canMove: true,
      partyCount: 0,
      flowSummary: "Next goal: Continue story",
      flowNextGoal: "Continue story",
      flowCompletionTarget: "Beat Mt. Silver",
    };

    const request = buildLocalOllamaActionRequest({
      model: "gemma-local",
      input: {
        session: {
          baseUrl: "http://127.0.0.1:3000",
          sessionId: "test",
        },
        overallGoal: "Play honest main-story Pokemon Crystal progress.",
        immediateGoal: "Play honest main-story Pokemon Crystal progress.",
        taskmasterModel: "ollama/gemma-local",
        playerModel: "ollama/gemma-local",
        supervisorMaxSteps: 20,
        playerMaxSteps: 12,
        autoSuspend: true,
        includeObservationCheckpoint: true,
      },
      status,
      observation: "OVERWORLD\nHOTSPOTS\nD Warp (1N 6E)",
      mapInfo: JSON.stringify({
        player: { coords: { x: 3, y: 3 } },
        hotspots: [{ type: "warp", coords: { x: 15, y: 1 } }],
      }),
      recentEvents: JSON.stringify({
        events: [{ action: "move:up:2", changed: false, reason: "blocked" }],
      }),
      rejectedActionNote: "move:right is invalid because that square is blocked.",
    });

    expect(request.response_format.type).toBe("json_schema");
    expect(request.response_format.json_schema.strict).toBe(true);
    expect(JSON.stringify(request.response_format.json_schema.schema)).toContain("type_text");
    expect(JSON.stringify(request.response_format.json_schema.schema)).toContain("additionalProperties");
    expect(request.messages[0].content).toContain("After a failed or blocked move");
    expect(request.messages[0].content).toContain("Never wait");
    expect(request.messages[0].content).toContain("Do not wait for user input");
    expect(request.messages[0].content).toContain("route through visible floor tiles");
    expect(request.messages[0].content).toContain("Utility and object hotspots are not story route targets");
    expect(request.messages[1].content).toContain("Map info:");
    expect(request.messages[1].content).toContain("move:up:2");
    expect(request.messages[1].content).toContain("Rejected candidates forbidden for this live state:");
    expect(request.messages[1].content).toContain("move:right is invalid");
    expect(request.messages[1].content.indexOf("Map info:")).toBeLessThan(
      request.messages[1].content.indexOf("Observation:"),
    );
  });

  it("builds Azure Responses requests with required nullable parameter fields", () => {
    const request = buildAzureOpenAIActionRequest({
      model: "gpt-5.4-mini",
      input: {
        session: {
          baseUrl: "http://127.0.0.1:3000",
          sessionId: "test",
        },
        overallGoal: "Play honest main-story Pokemon Crystal progress.",
        immediateGoal: "Play honest main-story Pokemon Crystal progress.",
        taskmasterModel: "azure-openai/gpt-5.4-mini",
        playerModel: "azure-openai/gpt-5.4-mini",
        supervisorMaxSteps: 20,
        playerMaxSteps: 12,
        autoSuspend: true,
        includeObservationCheckpoint: true,
      },
      status: {
        mode: "main_menu",
        map: "MAIN MENU",
        mapId: "main_menu",
        badges: 0,
        canMove: false,
        blockedReason: "main_menu",
        partyCount: 0,
        flowSummary: "Next goal: Continue story",
        flowNextGoal: "Continue story",
        flowCompletionTarget: "Beat Mt. Silver",
      },
      observation: "MAIN MENU\nMENU\nNEW GAME",
      mapInfo: "{}",
      recentEvents: JSON.stringify({ events: [] }),
    }) as {
      text: {
        format: {
          schema: {
            properties: {
              parameters: {
                required: string[];
                properties: Record<string, { type: unknown; enum?: unknown[] }>;
              };
            };
          };
        };
      };
    };

    const parameters = request.text.format.schema.properties.parameters;
    expect(parameters.required).toEqual(Object.keys(parameters.properties));
    expect(parameters.properties.button.type).toContain("null");
    expect(parameters.properties.button.enum).toContain(null);
    expect(parameters.properties.steps.type).toContain("null");
  });

  it("builds direct OpenAI Responses requests with strict structured output", () => {
    const request = buildOpenAIDirectActionRequest({
      model: "gpt-5.4-mini",
      input: {
        session: {
          baseUrl: "http://127.0.0.1:3000",
          sessionId: "test",
        },
        overallGoal: "Play honest main-story Pokemon Crystal progress.",
        immediateGoal: "Play honest main-story Pokemon Crystal progress.",
        taskmasterModel: "openai-direct/gpt-5.4-mini",
        playerModel: "openai-direct/gpt-5.4-mini",
        supervisorMaxSteps: 20,
        playerMaxSteps: 12,
        autoSuspend: true,
        includeObservationCheckpoint: true,
      },
      status: {
        mode: "main_menu",
        map: "MAIN MENU",
        mapId: "main_menu",
        badges: 0,
        canMove: false,
        blockedReason: "main_menu",
        partyCount: 0,
        flowSummary: "Next goal: Continue story",
        flowNextGoal: "Continue story",
        flowCompletionTarget: "Beat Mt. Silver",
      },
      observation: "MAIN MENU\nMENU\nNEW GAME",
      mapInfo: "{}",
      recentEvents: JSON.stringify({ events: [] }),
    }) as {
      max_output_tokens: number;
      text: { format: { type: string; strict: boolean; schema: { properties: { parameters: { required: string[] } } } } };
    };

    expect(request.max_output_tokens).toBe(160);
    expect(request.text.format.type).toBe("json_schema");
    expect(request.text.format.strict).toBe(true);
    expect(request.text.format.schema.properties.parameters.required).toContain("button");
  });

  it("builds Anthropic tool-use requests for one forced action", () => {
    const request = buildAnthropicActionRequest({
      model: "claude-sonnet-4-5",
      input: {
        session: {
          baseUrl: "http://127.0.0.1:3000",
          sessionId: "test",
        },
        overallGoal: "Play honest main-story Pokemon Crystal progress.",
        immediateGoal: "Play honest main-story Pokemon Crystal progress.",
        taskmasterModel: "anthropic/claude-sonnet-4-5",
        playerModel: "anthropic/claude-sonnet-4-5",
        supervisorMaxSteps: 20,
        playerMaxSteps: 12,
        autoSuspend: true,
        includeObservationCheckpoint: true,
      },
      status: {
        mode: "main_menu",
        map: "MAIN MENU",
        mapId: "main_menu",
        badges: 0,
        canMove: false,
        blockedReason: "main_menu",
        partyCount: 0,
        flowSummary: "Next goal: Continue story",
        flowNextGoal: "Continue story",
        flowCompletionTarget: "Beat Mt. Silver",
      },
      observation: "MAIN MENU\nMENU\nNEW GAME",
      mapInfo: "{}",
      recentEvents: JSON.stringify({ events: [] }),
    }) as {
      tools: Array<{ name: string; input_schema: { properties: { action: { enum: string[] } } } }>;
      tool_choice: { type: string; name: string };
      messages: Array<{ role: string; content: string }>;
    };

    expect(request.tools[0].name).toBe("pokemon_crystal_action");
    expect(request.tool_choice).toEqual({ type: "tool", name: "pokemon_crystal_action" });
    expect(request.tools[0].input_schema.properties.action.enum).toContain("move");
    expect(request.messages[0].content).toContain("Choose the next useful valid Pokemon Crystal action now.");
  });

  it("builds Google Gemini structured-output requests", () => {
    const request = buildGoogleActionRequest({
      model: "gemini-2.5-flash",
      input: {
        session: {
          baseUrl: "http://127.0.0.1:3000",
          sessionId: "test",
        },
        overallGoal: "Play honest main-story Pokemon Crystal progress.",
        immediateGoal: "Play honest main-story Pokemon Crystal progress.",
        taskmasterModel: "google/gemini-2.5-flash",
        playerModel: "google/gemini-2.5-flash",
        supervisorMaxSteps: 20,
        playerMaxSteps: 12,
        autoSuspend: true,
        includeObservationCheckpoint: true,
      },
      status: {
        mode: "main_menu",
        map: "MAIN MENU",
        mapId: "main_menu",
        badges: 0,
        canMove: false,
        blockedReason: "main_menu",
        partyCount: 0,
        flowSummary: "Next goal: Continue story",
        flowNextGoal: "Continue story",
        flowCompletionTarget: "Beat Mt. Silver",
      },
      observation: "MAIN MENU\nMENU\nNEW GAME",
      mapInfo: "{}",
      recentEvents: JSON.stringify({ events: [] }),
    }) as {
      contents: Array<{ parts: Array<{ text: string }> }>;
      generationConfig: { responseMimeType: string; responseJsonSchema: { properties: { action: { enum: string[] } } } };
    };

    expect(request.generationConfig.responseMimeType).toBe("application/json");
    expect(request.generationConfig.responseJsonSchema.properties.action.enum).toContain("press");
    expect(request.contents[0].parts[0].text).toContain("Never wait");
  });

  it("rejects a local action that exactly repeats a recent failed action from the same state", () => {
    const status: Status = {
      mode: "overworld",
      map: "SomeMap",
      mapId: "24:7",
      coords: [3, 3],
      facing: "up",
      badges: 0,
      canMove: true,
      partyCount: 0,
      flowSummary: "Next goal: Continue story",
      flowNextGoal: "Continue story",
      flowCompletionTarget: "Beat Mt. Silver",
    };

    expect(isLocalActionRejectedByRecentFailure({
      action: { action: "move", parameters: { direction: "up", steps: 1 }, reason: "try again" },
      status,
      recentEvents: JSON.stringify({
        events: [
          { action: "move:up:1", coords: [3, 3], changed: false, reason: "blocked" },
        ],
      }),
    })).toBe(true);

    expect(isLocalActionRejectedByRecentFailure({
      action: { action: "move", parameters: { direction: "right", steps: 1 }, reason: "different route" },
      status,
      recentEvents: JSON.stringify({
        events: [
          { action: "move:up:1", coords: [3, 3], changed: false, reason: "blocked" },
        ],
      }),
    })).toBe(false);

    expect(isLocalActionRejectedByRecentFailure({
      action: { action: "move", parameters: { direction: "right", steps: 2 }, reason: "repeat blocked direction" },
      status,
      recentEvents: JSON.stringify({
        events: [
          { action: "move:right:3", coords: [3, 3], changed: true, reason: "blocked" },
        ],
      }),
    })).toBe(true);

    expect(isLocalActionRejectedByRecentFailure({
      action: { action: "move", parameters: { direction: "right", steps: 2 }, reason: "walk into utility" },
      status: {
        ...status,
        facing: "right",
        interactionTarget: {
          coords: [5, 3],
          kind: "bg_event",
          label: "Utility",
          token: "P",
          hotspotType: "utility",
        },
      },
      recentEvents: JSON.stringify({ events: [] }),
    })).toBe(true);

    expect(isLocalActionRejectedByRecentFailure({
      action: { action: "press", parameters: { button: "A" }, reason: "activate warp" },
      status: {
        ...status,
        currentHotspot: {
          coords: [3, 3],
          label: "Warp",
          token: "D",
          hotspotType: "warp",
        },
      },
      recentEvents: JSON.stringify({ events: [] }),
    })).toBe(true);

    expect(isLocalActionRejectedByRecentFailure({
      action: { action: "press", parameters: { button: "A" }, reason: "repeat same NPC" },
      status: {
        ...status,
        interactionTarget: {
          coords: [3, 5],
          kind: "npc",
          label: "NPC",
          token: "N",
          hotspotType: "npc",
        },
        facing: "down",
      },
      recentEvents: JSON.stringify({
        events: [
          { action: "press:a:1", coords: [3, 3], summary: "text advance opened", changed: true },
          { action: "execute_macro:advance_dialog:2/8", coords: [3, 3], summary: "text advance closed", changed: true },
          { action: "press:a:1", coords: [3, 3], summary: "text advance opened", changed: true },
          { action: "execute_macro:advance_dialog:2/8", coords: [3, 3], summary: "text advance closed", changed: true },
        ],
      }),
    })).toBe(true);
  });

  it("does not frame non-elevated utility targets as the local prompt objective", () => {
    const status: Status = {
      mode: "overworld",
      map: "SomeMap",
      mapId: "24:7",
      coords: [3, 3],
      interactionTile: [5, 3],
      interactionTarget: {
        coords: [5, 3],
        kind: "bg_event",
        label: "Utility",
        token: "P",
        hotspotType: "utility",
      },
      facing: "right",
      badges: 0,
      canMove: true,
      partyCount: 0,
      flowSummary: "Next goal: Continue story",
      flowNextGoal: "Continue story",
      flowCompletionTarget: "Beat Mt. Silver",
    };

    expect(sanitizeLocalPromptStatus(status).interactionTarget).toBeUndefined();
    expect(sanitizeLocalPromptStatus({
      ...status,
      flowNextGoal: "Use Utility",
    }).interactionTarget).toBeDefined();

    expect(sanitizeLocalPromptMapInfo(JSON.stringify({
      hotspots: [
        { type: "utility", label: "Utility", token: "P" },
        { type: "warp", label: "Warp", token: "D" },
      ],
    }), status)).toBe(JSON.stringify({
      hotspots: [
        { type: "warp", label: "Warp", token: "D" },
      ],
    }));

    expect(sanitizeLocalPromptObservation([
      "OVERWORLD",
      "",
      "HOTSPOTS",
      "D Warp: visible door",
      "P Utility here",
      "S Sign (1S)",
      "N NPC (2E)",
      "",
      "FLOW",
      "Next goal: Continue story",
    ].join("\n"), status)).toBe([
      "OVERWORLD",
      "",
      "HOTSPOTS",
      "D Warp: visible door",
      "N NPC (2E)",
      "",
      "FLOW",
      "Next goal: Continue story",
    ].join("\n"));
  });

  it("does not frame a recently used reverse warp as the next route target when another exit is visible", () => {
    const status: Status = {
      mode: "overworld",
      map: "CurrentMap",
      mapId: "24:6",
      coords: [19, 1],
      facing: "up",
      badges: 0,
      canMove: true,
      partyCount: 0,
      flowSummary: "Next goal: Continue story",
      flowNextGoal: "Continue story",
      flowCompletionTarget: "Beat Mt. Silver",
    };
    const recentEvents = JSON.stringify({
      events: [
        { summary: "map:PreviousMap->CurrentMap", action: "move:up:1" },
      ],
    });

    expect(sanitizeLocalPromptMapInfo(JSON.stringify({
      hotspots: [
        { type: "warp", label: "Warp: Previous Map", token: "D" },
        { type: "warp", label: "Warp: Next Map", token: "D" },
        { type: "npc", label: "NPC", token: "N" },
      ],
    }), status, recentEvents)).toBe(JSON.stringify({
      hotspots: [
        { type: "warp", label: "Warp: Next Map", token: "D" },
        { type: "npc", label: "NPC", token: "N" },
      ],
    }));

    expect(sanitizeLocalPromptObservation([
      "OVERWORLD",
      "HOTSPOTS",
      "D Warp: Previous Map here",
      "D Warp: Next Map (7S)",
      "N NPC (3S)",
      "FLOW",
    ].join("\n"), status, recentEvents)).toBe([
      "OVERWORLD",
      "HOTSPOTS",
      "D Warp: Next Map (7S)",
      "N NPC (3S)",
      "FLOW",
    ].join("\n"));
  });

  it("does not frame a repeatedly exhausted NPC dialogue as the local prompt objective", () => {
    const status: Status = {
      mode: "overworld",
      map: "CurrentMap",
      mapId: "24:6",
      coords: [3, 3],
      interactionTarget: {
        coords: [3, 5],
        kind: "npc",
        label: "NPC",
        token: "N",
        hotspotType: "npc",
      },
      facing: "down",
      badges: 0,
      canMove: true,
      partyCount: 0,
      flowSummary: "Next goal: Continue story",
      flowNextGoal: "Continue story",
      flowCompletionTarget: "Beat Mt. Silver",
    };
    const recentEvents = JSON.stringify({
      events: [
        { action: "press:a:1", coords: [3, 3], summary: "text advance opened", changed: true },
        { action: "execute_macro:advance_dialog:2/8", coords: [3, 3], summary: "text advance closed", changed: true },
        { action: "press:a:1", coords: [3, 3], summary: "text advance opened", changed: true },
        { action: "execute_macro:advance_dialog:2/8", coords: [3, 3], summary: "text advance closed", changed: true },
      ],
    });

    expect(sanitizeLocalPromptStatus(status, recentEvents).interactionTarget).toBeUndefined();
    expect(sanitizeLocalPromptObservation([
      "OVERWORLD",
      "HOTSPOTS",
      "N NPC here",
      "D Warp: Route (4S)",
      "FLOW",
    ].join("\n"), status, recentEvents)).toBe([
      "OVERWORLD",
      "HOTSPOTS",
      "D Warp: Route (4S)",
      "FLOW",
    ].join("\n"));
  });
});

describe("streaming agent execution", () => {
  it("uses stream and consumes it so token callbacks can fire", async () => {
    const consumeStream = jest.fn().mockResolvedValue(undefined);
    const stream = jest.fn().mockResolvedValue({ consumeStream });

    const output = await consumeAgentStream(
      { stream } as never,
      "play the next turn",
      { maxSteps: 2, onChunk: jest.fn() } as never,
    );

    expect(stream).toHaveBeenCalledWith("play the next turn", expect.objectContaining({ maxSteps: 2 }));
    expect(consumeStream).toHaveBeenCalledTimes(1);
    expect(output).toEqual({ consumeStream });
  });
});
