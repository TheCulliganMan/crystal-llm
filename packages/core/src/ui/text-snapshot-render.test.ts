import { buildTextSnapshotLayout, buildTextSnapshotLines } from "./text-snapshot-render";

describe("buildTextSnapshotLines", () => {
  it("keeps legend lines attached to the viewport when rendering overworld overlays", () => {
    const lines = buildTextSnapshotLines({
      viewportLines: ["01 ..@.", "02 ####"],
      infoLines: ["Legend: @=Player .=Floor #=Wall", "Pos: (49,21)"],
      viewportTitle: "Overworld",
      infoTitle: "Legend",
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
      actionLog: [],
      marker: null,
    });

    expect(lines).toEqual([
      "OVERWORLD",
      "01 ..@.",
      "02 ####",
      "Legend: @=Player .=Floor #=Wall",
      "Pos: (49,21)",
    ]);
  });

  it("keeps non-legend info in its own section", () => {
    const lines = buildTextSnapshotLines({
      viewportLines: ["BATTLE"],
      infoLines: ["Turn 1"],
      viewportTitle: "Battle",
      infoTitle: "Info",
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
      actionLog: [],
      marker: null,
    });

    expect(lines).toEqual([
      "BATTLE",
      "",
      "INFO",
      "Turn 1",
    ]);
  });

  it("reuses the cached layout for the same snapshot object", () => {
    const snapshot = {
      viewportLines: ["BATTLE"],
      infoLines: ["Turn 1"],
      viewportTitle: "Battle",
      infoTitle: "Info",
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
      actionLog: [],
      marker: null,
    };

    const first = buildTextSnapshotLayout(snapshot);
    const second = buildTextSnapshotLayout(snapshot);

    expect(second).toBe(first);
  });
});
