import { MoveName } from "@pokecrystal/core/core/enums/move";
import { asmMoveDescriptionsLoader } from "@pokecrystal/core/core/asm-move-descriptions-loader";

describe("AsmMoveDescriptionsLoader", () => {
  it("parses move descriptions from the disassembly", () => {
    expect(asmMoveDescriptionsLoader.get(MoveName.TACKLE)).toBe(
      "A full-body charge\nattack."
    );
  });
});
