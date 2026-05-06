import { MoveName } from "@pokecrystal/core/core/enums/move";
import {
  asmMoveNamesLoader,
  getAsmMoveNameOrder,
  moveNameForId,
} from "@pokecrystal/core/core/asm-move-names-loader";
import { moveDisplayName } from "@pokecrystal/assets/content/move-names";

describe("AsmMoveNamesLoader", () => {
  it("covers the canonical order of moves", () => {
    const order = getAsmMoveNameOrder();
    expect(order.length).toBe(251);
    expect(order[0]).toBe(MoveName.POUND);
    expect(order.at(-1)).toBe(MoveName.BEAT_UP);
  });

  it("exposes ASM-approved labels", () => {
    expect(asmMoveNamesLoader.get(MoveName.GIGA_DRAIN)).toBe("GIGA DRAIN");
    expect(moveDisplayName(MoveName.GIGA_DRAIN)).toBe("GIGA DRAIN");
  });

  it("normalizes looser inputs back to canonical entries", () => {
    expect(moveDisplayName("Solar Beam")).toBe("SOLARBEAM");
    expect(moveDisplayName("Sand-Attack")).toBe("SAND-ATTACK");
  });

  it("resolves numeric move ids", () => {
    const order = getAsmMoveNameOrder();
    expect(moveNameForId(0)).toBe(MoveName.POUND);
    expect(moveDisplayName(0)).toBe("POUND");
    expect(moveNameForId(order.length)).toBeUndefined();
  });

  it("throws on unknown names", () => {
    expect(() => moveDisplayName("Not A Move")).toThrow(/Unknown move/);
  });
});
