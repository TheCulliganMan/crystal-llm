import {
  buildInteractMicroAdjustActions,
  ExecuteMacroSchema,
  runViewportApproachMacro,
} from "./input";

describe("ExecuteMacroSchema", () => {
  it("does not accept wait actions on the MCP macro surface", () => {
    expect(
      ExecuteMacroSchema.safeParse({
        actions: [{ type: "wait", frames: 8 }],
      }).success
    ).toBe(false);
  });
});

describe("buildInteractMicroAdjustActions", () => {
  it("builds press-a retries with paired opposite micro-adjust moves to limit drift", () => {
    const actions = buildInteractMicroAdjustActions(5, 1);

    expect(actions).toEqual([
      { type: "button", value: "a", times: 1, hold_frames: 1, delay_frames: 1 },
      { type: "move", value: "up", times: 1, hold_frames: 1, delay_frames: 0 },
      { type: "button", value: "a", times: 1, hold_frames: 1, delay_frames: 1 },
      { type: "move", value: "down", times: 1, hold_frames: 1, delay_frames: 0 },
      { type: "button", value: "a", times: 1, hold_frames: 1, delay_frames: 1 },
      { type: "move", value: "right", times: 1, hold_frames: 1, delay_frames: 0 },
      { type: "button", value: "a", times: 1, hold_frames: 1, delay_frames: 1 },
      { type: "move", value: "left", times: 1, hold_frames: 1, delay_frames: 0 },
      { type: "button", value: "a", times: 1, hold_frames: 1, delay_frames: 1 },
    ]);
  });
});

const buildOverworldSnapshot = (rows: string[][]): string => {
  const width = Math.max(...rows.map((row) => row.length));
  const headerCols = Array.from({ length: width }, (_, index) => String(index).padStart(2, "0")).join(" ");
  const body = rows
    .map((row, y) => `${String(y).padStart(2, "0")} ${row.join(" ")}`)
    .join("\n");
  return `OVERWORLD\n   ${headerCols}\n${body}\n\n`;
};

describe("runViewportApproachMacro", () => {
  it("re-observes every step, reacquires drifting targets, then micro-adjusts to face and press A", async () => {
    let playerX = 1;
    let targetX = 4;
    let facing: "left" | "right" = "right";
    let forcedLeftAtAdjacency = true;
    const moveCalls: string[] = [];
    const pressCalls: string[] = [];
    let observeCalls = 0;

    const session = {
      observeText: () => {
        observeCalls += 1;
        const row = [".", ".", ".", ".", ".", ".", "."];
        if (playerX === targetX - 1 && forcedLeftAtAdjacency) {
          row[playerX] = "@<";
        } else {
          row[playerX] = facing === "right" ? "@>" : "@<";
        }
        row[targetX] = "N>";
        return buildOverworldSnapshot([["#", "#", "#", "#", "#", "#", "#"], row, ["#", "#", "#", "#", "#", "#", "#"]]);
      },
      move: async (direction: "up" | "down" | "left" | "right") => {
        moveCalls.push(direction);
        if (direction === "right") {
          facing = "right";
          if (playerX + 1 < targetX) {
            playerX += 1;
          } else if (playerX + 1 === targetX && forcedLeftAtAdjacency) {
            forcedLeftAtAdjacency = false;
          }
        }
        if (moveCalls.length === 1) {
          targetX = 5;
        }
        return { result: { ok: true, changed: true, events: [`move:${direction}`] }, snapshotText: "" };
      },
      press: async (button: "a") => {
        pressCalls.push(button);
        return { result: { ok: true, changed: true, events: ["press:a"] }, snapshotText: "" };
      },
    };

    const result = await runViewportApproachMacro(session, {
      targetToken: "N",
      maxSteps: 10,
      maxObserves: 20,
      maxTries: 4,
      pressA: true,
    });

    expect(moveCalls).toEqual(["right", "right", "right", "right"]);
    expect(pressCalls).toEqual(["a"]);
    expect(observeCalls).toBeGreaterThan(moveCalls.length);
    expect(result.result.events).toContain("press:a");
  });

  it("stops at max_steps when no progress is possible", async () => {
    const moveCalls: string[] = [];
    const session = {
      observeText: () =>
        buildOverworldSnapshot([
          ["#", "#", "#", "#", "#"],
          ["#", "@>", "#", "N>", "#"],
          ["#", "#", "#", "#", "#"],
        ]),
      move: async (direction: "up" | "down" | "left" | "right") => {
        moveCalls.push(direction);
        return { result: { ok: false, changed: false, reason: "blocked", events: [`blocked:${direction}`] }, snapshotText: "" };
      },
      press: async (_button: "a") => ({ result: { ok: true, changed: true, events: ["press:a"] }, snapshotText: "" }),
    };

    const result = await runViewportApproachMacro(session, {
      targetToken: "N",
      maxSteps: 2,
      maxObserves: 10,
      maxTries: 2,
      pressA: true,
    });

    expect(moveCalls).toEqual(["right", "right"]);
    expect(result.result.reason).toBe("blocked");
    expect(result.result.events).toContain("blocked:right");
  });

  it("routes around a diagonal obstruction instead of oscillating on greedy axis choices", async () => {
    let player = { x: 1, y: 1 };
    const moveCalls: string[] = [];
    const session = {
      observeText: () =>
        buildOverworldSnapshot([
          ["#", "#", "#", "#", "#", "#"],
          ["#", player.x === 1 && player.y === 1 ? "@>" : ".", ".", ".", ".", "#"],
          ["#", ".", "#", "D^", ".", "#"],
          ["#", ".", ".", ".", ".", "#"],
          ["#", "#", "#", "#", "#", "#"],
        ].map((row, y) =>
          row.map((tile, x) => (x === player.x && y === player.y ? (player.y === 2 ? "@v" : "@>") : tile))
        )),
      move: async (direction: "up" | "down" | "left" | "right") => {
        moveCalls.push(direction);
        if (direction === "right" && player.x === 1 && player.y === 1) {
          player = { x: 2, y: 1 };
        } else if (direction === "right" && player.x === 2 && player.y === 1) {
          player = { x: 3, y: 1 };
        } else if (direction === "down" && player.x === 3 && player.y === 1) {
          player = { x: 3, y: 2 };
        }
        return { result: { ok: true, changed: true, events: [`move:${direction}`] }, snapshotText: "" };
      },
      press: async (_button: "a") => ({ result: { ok: true, changed: true, events: ["press:a"] }, snapshotText: "" }),
    };

    const result = await runViewportApproachMacro(session, {
      targetToken: "D",
      maxSteps: 6,
      maxObserves: 12,
      maxTries: 2,
      pressA: false,
    });

    expect(moveCalls).toEqual(["right", "right", "down"]);
    expect(result.result.ok).toBe(true);
    expect(result.result.events).toContain("move:down");
  });

  it("stops after a successful warp map transition instead of chasing the next visible warp", async () => {
    let map = "PlayersHouse2F";
    let player = { x: 5, y: 1 };
    const moveCalls: string[] = [];
    const session = {
      observeText: () =>
        buildOverworldSnapshot([
          ["#", "#", "#", "#", "#", "#", "#", "#"],
          [".", ".", ".", ".", ".", ".", ".", "D"],
          [".", ".", ".", ".", ".", ".", ".", "."],
        ].map((row, y) =>
          row.map((tile, x) => (x === player.x && y === player.y ? "@>" : tile))
        )),
      status: async () => ({ map }),
      move: async (direction: "up" | "down" | "left" | "right") => {
        moveCalls.push(direction);
        if (direction === "right") {
          if (player.x === 5) {
            player = { x: 6, y: 1 };
          } else {
            player = { x: 7, y: 1 };
            map = "PlayersHouse1F";
          }
        }
        return { result: { ok: true, changed: true, events: [`move:${direction}`] }, snapshotText: "" };
      },
      press: async (_button: "a") => ({ result: { ok: true, changed: true, events: ["press:a"] }, snapshotText: "" }),
    };

    const result = await runViewportApproachMacro(session, {
      targetToken: "Door",
      maxSteps: 5,
      maxObserves: 10,
      maxTries: 2,
      pressA: false,
    });

    expect(moveCalls).toEqual(["right", "right"]);
    expect(result.result.ok).toBe(true);
    expect(result.result.events).toContain("move:right");
  });
});

describe("ExecuteMacroSchema approach_target", () => {
  it("requires target_token for the approach_target macro", () => {
    const parsed = ExecuteMacroSchema.safeParse({ macro: "approach_target", max_steps: 10 });
    expect(parsed.success).toBe(false);
  });
});
