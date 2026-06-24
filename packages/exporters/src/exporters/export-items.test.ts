import fs from "fs";
import os from "os";
import path from "path";
import { exportItems } from "./export-items";

var mockDisassemblyRoot = "";
var mockAssetsRoot = "";

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => mockDisassemblyRoot || "/mock/pokecrystal",
  getAssetsRoot: () => mockAssetsRoot || "/mock/assets",
}));

const writeFile = (filePath: string, content: string): void => {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content);
};

const unusedAttributeRow = (name: string): string =>
  [`; ${name}`, "\titem_attribute 0, HELD_NONE, 0, NO_LIMITS, ITEM, ITEMMENU_NOUSE, ITEMMENU_NOUSE"].join("\n");

const completeAttributeTable = (firstName = "MASTER_BALL"): string => {
  const rows = [
    [`; ${firstName}`, "\titem_attribute 0, HELD_NONE, 0, CANT_SELECT, BALL, ITEMMENU_NOUSE, ITEMMENU_CLOSE"].join("\n"),
  ];
  for (let index = 1; index < 256; index += 1) {
    rows.push(unusedAttributeRow(`ITEM_${index.toString(16).toUpperCase().padStart(2, "0")}`));
  }
  return `${rows.join("\n")}\n`;
};

const completeDescriptionTable = (missingLabel?: string): string => {
  const pointers = ["MasterBallDesc"];
  for (let index = 1; index < 255; index += 1) {
    pointers.push(`Unused${index}Desc`);
  }
  const labels = pointers
    .filter((label) => label !== missingLabel)
    .map((label) => `${label}:\n\tdb "${label} text.@"`);
  return [`ItemDescriptions:`, ...pointers.map((label) => `\tdw ${label}`), "", ...labels, ""].join("\n");
};

describe("exportItems", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-items-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");

    writeFile(path.join(mockDisassemblyRoot, "constants", "item_constants.asm"), "");
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "descriptions.asm"), completeDescriptionTable());
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("requires the item attribute table to define every byte-sized item slot", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"),
      ["; MASTER_BALL", "\titem_attribute 0, HELD_NONE, 0, NO_LIMITS, ITEM, ITEMMENU_NOUSE, ITEMMENU_NOUSE", ""].join("\n")
    );

    expect(() => exportItems()).toThrow("Item attribute table must contain exactly 256 rows, found 1");
  });

  it("requires the description pointer table to define every nonzero item id", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable());
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "descriptions.asm"),
      'ItemDescriptions:\n\tdw MasterBallDesc\n\nMasterBallDesc:\n\tdb "The best BALL.@"\n'
    );

    expect(() => exportItems()).toThrow("Item description pointer table must contain exactly 255 rows, found 1");
  });

  it("requires each description pointer to reference an authored label", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable());
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "descriptions.asm"), completeDescriptionTable("Unused1Desc"));

    expect(() => exportItems()).toThrow("missing item description label Unused1Desc for item slot 1");
  });

  it("requires authored item effects to use exact ASM symbols", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("BLACK_GLASSES"));

    expect(() => exportItems()).toThrow("missing authored item effect for item slot 0");
  });
});
