import {
  STORY_STEPS,
  buildMermaidDiagram,
  calculateCompletionPercent,
  getAvailableStepIds,
  validateStoryGraph,
} from "@/app/game-corner/progress-tracker";

describe("progress-tracker helpers", () => {
  it("builds Mermaid output with completed/ready/todo class tags", () => {
    const output = buildMermaidDiagram(STORY_STEPS, ["starter"]);

    expect(output).toContain("flowchart TD");
    expect(output).toContain("class starter done;");
    expect(output).toContain("class mr-pokemon ready;");
    expect(output).toContain("class red-defeated todo;");
    expect(output).toContain("starter --> mr-pokemon");
    expect(output).toContain("mr-pokemon --> mom-bank");
    expect(output).toContain("mom-bank --> violet-badge");
  });

  it("calculates completion percentages from known steps", () => {
    expect(calculateCompletionPercent(STORY_STEPS, [])).toBe(0);
    expect(calculateCompletionPercent(STORY_STEPS, ["starter", "mr-pokemon", "missing-id"]))
      .toBe(Math.round((2 / STORY_STEPS.length) * 100));
  });

  it("exposes only currently reachable steps", () => {
    expect(getAvailableStepIds(STORY_STEPS, [])).toEqual(["starter"]);
    expect(getAvailableStepIds(STORY_STEPS, ["starter"])).toEqual(["mr-pokemon"]);
    expect(getAvailableStepIds(STORY_STEPS, ["starter", "mr-pokemon"])).toEqual(["mom-bank"]);
  });

  it("validates the graph as acyclic with a route to Red", () => {
    const validation = validateStoryGraph(STORY_STEPS);

    expect(validation.isAcyclic).toBe(true);
    expect(validation.hasDanglingReferences).toBe(false);
    expect(validation.isRedReachable).toBe(true);
    expect(validation.orderedStepIds).toContain("red-defeated");
  });
});
