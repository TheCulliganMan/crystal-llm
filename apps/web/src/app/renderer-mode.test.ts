import {
  getNextRendererMode,
  getRendererModeActionLabel,
  getRendererModeLabel,
  rendererModeCycle,
} from "./renderer-mode";

describe("renderer-mode helpers", () => {
  it("cycles through the configured renderer modes", () => {
    expect(getNextRendererMode("tile")).toBe("both");
    expect(getNextRendererMode("both")).toBe("text");
    expect(getNextRendererMode("text")).toBe("tile");
    expect(rendererModeCycle).toEqual(["tile", "both", "text"]);
  });

  it("exposes a label for each renderer mode", () => {
    expect(getRendererModeLabel("tile")).toBe("Tile View");
    expect(getRendererModeLabel("both")).toBe("Tile + Text");
    expect(getRendererModeLabel("text")).toBe("Text View");
  });

  it("exposes an action label for each renderer mode", () => {
    expect(getRendererModeActionLabel("tile")).toBe("Show Tile + Text");
    expect(getRendererModeActionLabel("both")).toBe("Show Text View");
    expect(getRendererModeActionLabel("text")).toBe("Show Tile View");
  });
});
