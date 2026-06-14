import { compactAgentGoal, derivePlayerMaxSteps, shouldRestartWorkflowRun, summarizeWorkflowFailure } from "./agent.js";

describe("agent runtime budgets", () => {
  it("lets the player batch use the full graph cycle budget up to the hard cap", () => {
    expect(derivePlayerMaxSteps(4)).toBe(4);
    expect(derivePlayerMaxSteps(8)).toBe(8);
    expect(derivePlayerMaxSteps(20)).toBe(12);
  });

  it("restarts the workflow cleanly when a prior run is no longer suspended", () => {
    expect(shouldRestartWorkflowRun(new Error("This workflow run was not suspended"))).toBe(true);
    expect(shouldRestartWorkflowRun(new Error("Timed out waiting for workflow resume."))).toBe(false);
    expect(shouldRestartWorkflowRun(new Error("something else failed"))).toBe(false);
  });

  it("summarizes serialized workflow step failures", () => {
    expect(
      summarizeWorkflowFailure({
        status: "failed",
        steps: {
          "play-batch": {
            status: "failed",
            error: {
              name: "Error",
              message: "Taskmaster batch ended without delegating gameplay to the player agent.",
            },
          },
        },
      }),
    ).toBe("Taskmaster batch ended without delegating gameplay to the player agent.");
  });

  it("compacts oversized intervention goals for small local model contexts", () => {
    const goal = [
      "continue beating the game",
      "Professor Culligan's Intervention:",
      "Professor Culligan paused autonomous play and manually controlled the live game for a short stretch.",
      ...Array.from({ length: 30 }, (_, index) =>
        `${index + 1}. press A (key "z") -> {"context":{"very":"large","payload":"${"x".repeat(200)}"}}; mode=battle map=Route29 coords=67,13`
      ),
      "YOU MUST MAKE A CHOICE using live MCP evidence.",
    ].join("\n");

    const compact = compactAgentGoal(goal);

    expect(compact.length).toBeLessThanOrEqual(1700);
    expect(compact).toContain("Professor Culligan's Intervention");
    expect(compact).toContain("YOU MUST MAKE A CHOICE");
    expect(compact).not.toContain("\"payload\"");
  });
});
