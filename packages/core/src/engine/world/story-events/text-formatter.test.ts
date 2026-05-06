import { createInitialGameState } from "@pokecrystal/core/core/state";
import { TextFormatter } from "./text-formatter";

describe("TextFormatter", () => {
  it("preserves the single-tile Poké glyph in formatted text", () => {
    const gameState = createInitialGameState();
    const formatter = new TextFormatter(gameState);

    const formatted = formatter.formatText("A whole collection of <POKE>MON");

    expect(formatted).toBe("A whole collection of POK\u00e9MON");
  });

  it("replaces @ with string buffers in numeric order", () => {
    const gameState = createInitialGameState();
    const formatter = new TextFormatter(gameState);
    formatter.stringBuffers = {
      STRING_BUFFER_2: "TWO",
      STRING_BUFFER_1: "ONE",
    };

    expect(formatter.formatText("@ @")).toBe("ONE TWO");
  });

  it("resolves ASM decimal interpolation with map-local script constants", () => {
    const gameState = createInitialGameState();
    const formatter = new TextFormatter(gameState, {
      getMapName: () => "Route43Gate",
    });

    expect(formatter.formatText("The toll is ¥{d:ROUTE43GATE_TOLL}")).toBe("The toll is ¥1000");
  });
});
