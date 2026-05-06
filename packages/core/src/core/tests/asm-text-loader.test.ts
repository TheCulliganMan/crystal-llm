jest.mock("../asset-manifest", () => {
  const fs = jest.requireActual("fs");
  return {
    assetExists: (filePath: string) => fs.existsSync(filePath),
    listAssetDir: (dirPath: string) =>
      fs.existsSync(dirPath) ? fs.readdirSync(dirPath) : [],
  };
});

import fs from "fs";
import path from "path";
import os from "os";
import { AsmTextLoader } from "../asm-text-loader";

describe("AsmTextLoader", () => {
  let tempDir: string;
  let dataDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-ts-test-"));
    dataDir = path.join(tempDir, "data");
    fs.mkdirSync(dataDir, { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  const writeTextJson = (payload: Record<string, string>) => {
    fs.writeFileSync(
      path.join(dataDir, "asm_text.json"),
      JSON.stringify(payload)
    );
  };

  it("loads a simple text entry", () => {
    writeTextJson({ DummyText: "Hello, world!" });
    const loader = new AsmTextLoader(tempDir, dataDir);
    expect(loader.get("DummyText")).toBe("Hello, world!");
  });

  it("loads entries containing line breaks", () => {
    writeTextJson({
      DoubleColonText: `Would you like to
save the game?`,
    });
    const loader = new AsmTextLoader(tempDir, dataDir);
    expect(loader.get("DoubleColonText")).toBe(`Would you like to
save the game?`);
  });

  it("loads multi-line text entries", () => {
    writeTextJson({
      MultiLineText: `This is the first line.
This is the second.`,
    });
    const loader = new AsmTextLoader(tempDir, dataDir);
    expect(loader.get("MultiLineText"))
      .toBe(`This is the first line.
This is the second.`);
  });

  it("loads entries containing string buffer markers", () => {
    writeTextJson({
      CaughtAskNicknameText: `Give a nickname to
the <STRING_BUFFER_1> you
received?`,
    });
    const loader = new AsmTextLoader(tempDir, dataDir);
    expect(loader.get("CaughtAskNicknameText"))
      .toBe(`Give a nickname to
the <STRING_BUFFER_1> you
received?`);
  });

  it("loads entries with paragraph breaks", () => {
    writeTextJson({
      ParaText: `First paragraph.

Second paragraph.`,
    });
    const loader = new AsmTextLoader(tempDir, dataDir);
    expect(loader.get("ParaText"))
      .toBe(`First paragraph.

Second paragraph.`);
  });

  it("should return an empty string for a missing label", () => {
    writeTextJson({ ExistingText: "ok" });
    const loader = new AsmTextLoader(tempDir, dataDir);
    expect(loader.get("NonExistentLabel")).toBe("");
  });

  it("loads multiple entries from the export", () => {
    writeTextJson({
      Text1: "Text from file 1.",
      Text2: "Text from file 2.",
    });
    const loader = new AsmTextLoader(tempDir, dataDir);
    expect(loader.get("Text1")).toBe("Text from file 1.");
    expect(loader.get("Text2")).toBe("Text from file 2.");
  });
});
