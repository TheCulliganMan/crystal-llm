import { __test__scriptStatusSnapshotEquals } from "@pokecrystal/core/engine/world/overworld/overworld";

describe("Overworld debug snapshot compare", () => {
  it("compares script status snapshots by value", () => {
    const a: [boolean, number, number, boolean, number, boolean, boolean] = [true, 1, 1, false, 0, false, false];
    const b: [boolean, number, number, boolean, number, boolean, boolean] = [true, 1, 1, false, 0, false, false];
    const c: [boolean, number, number, boolean, number, boolean, boolean] = [true, 1, 2, false, 0, false, false];

    expect(__test__scriptStatusSnapshotEquals(a, b)).toBe(true);
    expect(__test__scriptStatusSnapshotEquals(a, c)).toBe(false);
    expect(__test__scriptStatusSnapshotEquals(a, null)).toBe(false);
    expect(__test__scriptStatusSnapshotEquals(null, null)).toBe(true);
  });
});

