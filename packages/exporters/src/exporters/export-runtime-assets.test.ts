import fs from "fs";
import os from "os";
import path from "path";
import { exportFleeMons, exportPermanentPhoneNumbers, exportRuntimeAssets, timeTokenToMask } from "./export-runtime-assets";

let mockDisassemblyRoot = "";
let mockAssetsRoot = "";

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => mockDisassemblyRoot,
  getAssetsRoot: () => mockAssetsRoot,
}));

const writeFile = (filePath: string, content: string): void => {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content);
};

describe("exportRuntimeAssets", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");

    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "flee_mons.asm"),
      [
        "AlwaysFleeMons:",
        "\tdb RAIKOU",
        "\tdb ENTEI",
        "\tdb -1",
        "",
        "OftenFleeMons:",
        "\tdb DELIBIRD ; comment",
        "\tdb -1",
        "",
        "SometimesFleeMons:",
        "\tdb MAGNEMITE",
        "\tdb -1",
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "marts.asm"),
      ["MartCherrygrove:", "\tdb 2", "\tdb POTION", "\tdb ANTIDOTE", "\tdb -1", ""].join("\n")
    );
    writeFile(path.join(mockDisassemblyRoot, "constants", "item_constants.asm"), "");
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "pokemon_constants.asm"),
      [
        "\tconst_def 1",
        "\tconst CHIKORITA",
        "\tconst MR__MIME",
        "\tconst UNOWN_A",
        "\tconst EGG",
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "phone_constants.asm"),
      [
        "\tconst PHONE_00",
        "\tconst PHONE_MOM",
        "\tDEF NUM_PHONE_CONTACTS EQU const_value",
        "; SpecialPhoneCallList indexes (see data/phone/special_calls.asm)",
        "\tconst_def",
        "\tconst SPECIALCALL_NONE",
        "\tconst SPECIALCALL_MASTERBALL",
        "\tDEF NUM_SPECIALCALLS EQU const_value - 1",
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "npc_trade_constants.asm"),
      ["\tconst NPC_TRADE_MIKE", "\tconst NPC_TRADE_KYLE", ""].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "phone_contacts.asm"),
      [
        "\tphone TRAINER_NONE, 0, 0, 0, 0, 0, 0",
        "\tphone TRAINER_NONE, PHONECONTACT_MOM, PLAYERS_HOUSE_1F, ANYTIME, MomPhoneCalleeScript, 0, 0",
        "",
      ].join("\n")
    );
    writeFile(path.join(mockDisassemblyRoot, "data", "phone", "permanent_numbers.asm"), "\tdb PHONE_MOM\n\tdb -1\n");
    writeFile(path.join(mockDisassemblyRoot, "data", "phone", "non_trainer_names.asm"), '.mom: db "MOM:@"\n');
    writeFile(path.join(mockDisassemblyRoot, "constants", "trainer_constants.asm"), "trainerclass TRAINER_NONE\n");
    writeFile(path.join(mockDisassemblyRoot, "data", "trainers", "class_names.asm"), "");
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "special_pointers.asm"),
      ["SpecialsPointers:", "\tadd_special FadeOutMusic", "\tadd_special HealParty", ""].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "pokemon", "bills_pc.asm"),
      'PCString_ChooseaPKMN: db "Choose a <PK><MN>.@"\n'
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "pokemon", "menu_icons.asm"),
      ["\tdb ICON_CHIKORITA ; CHIKORITA", "\tdb ICON_HUMANSHAPE ; MR__MIME", ""].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "pokemon", "dex_entries", "chikorita.asm"),
      [
        '\tdb "LEAF@"',
        "\tdw 211, 140 ; height, weight",
        '\tdb "A sweet aroma"',
        '\tnext "gently wafts@"',
        '\tpage "from the leaf"',
        '\tnext "on its head.@"',
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "pokemon", "dex_entries", "mr__mime.asm"),
      ['\tdb "BARRIER@"', "\tdw 403, 1200 ; height, weight", '\tdb "It mimics@"', ""].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tframe 1, 07", "\tsetrepeat 2", "\tframe 0, 05", "\tdorepeat 1", "\tendanim", ""].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "mr__mime", "anim.asm"),
      ["\tframe 0, 05", "\tendanim", ""].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "unown_a", "anim.asm"),
      ["\tframe 0, 05", "\tendanim", ""].join("\n")
    );
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports required runtime JSON assets including flee_mons.json", () => {
    exportRuntimeAssets();

    const dataDir = path.join(mockAssetsRoot, "data");
    const fleeMons = JSON.parse(fs.readFileSync(path.join(dataDir, "flee_mons.json"), "utf8"));
    const pokedexEntries = JSON.parse(fs.readFileSync(path.join(dataDir, "pokedex_entries.json"), "utf8"));
    const frontpicAnimations = JSON.parse(fs.readFileSync(path.join(dataDir, "pokemon_frontpic_anim.json"), "utf8"));
    const phoneContacts = JSON.parse(fs.readFileSync(path.join(dataDir, "phone_contacts.json"), "utf8"));
    const permanentPhoneNumbers = JSON.parse(fs.readFileSync(path.join(dataDir, "permanent_phone_numbers.json"), "utf8"));
    const specialPhoneCalls = JSON.parse(fs.readFileSync(path.join(dataDir, "special_phone_calls.json"), "utf8"));
    const npcTrades = JSON.parse(fs.readFileSync(path.join(dataDir, "npc_trades.json"), "utf8"));
    const specialRoutines = JSON.parse(fs.readFileSync(path.join(dataDir, "special_routines.json"), "utf8"));

    expect(fleeMons).toEqual({
      always: ["RAIKOU", "ENTEI"],
      often: ["DELIBIRD"],
      sometimes: ["MAGNEMITE"],
    });
    expect(pokedexEntries[0]).toMatchObject({
      species: "CHIKORITA",
      classification: "LEAF",
      pages: ["A sweet aroma @ gently wafts", "from the leaf @ on its head."],
    });
    expect(pokedexEntries).toContainEqual(
      expect.objectContaining({
        species: "MR__MIME",
        classification: "BARRIER",
      })
    );
    expect(frontpicAnimations.CHIKORITA.commands).toEqual([
      { kind: "frame", frame: 1, duration: 7 },
      { kind: "setrepeat", count: 2 },
      { kind: "frame", frame: 0, duration: 5 },
      { kind: "dorepeat", target: 1 },
      { kind: "endanim" },
    ]);
    expect(frontpicAnimations.MR__MIME.commands).toEqual([
      { kind: "frame", frame: 0, duration: 5 },
      { kind: "endanim" },
    ]);
    expect(frontpicAnimations.UNOWN_A.commands).toEqual([
      { kind: "frame", frame: 0, duration: 5 },
      { kind: "endanim" },
    ]);
    const menuIcons = JSON.parse(fs.readFileSync(path.join(dataDir, "menu_icons.json"), "utf8"));
    expect(menuIcons.MR__MIME).toBe("ICON_HUMANSHAPE");
    const marts = JSON.parse(fs.readFileSync(path.join(dataDir, "marts.json"), "utf8"));
    expect(marts).toEqual({
      MART_CHERRYGROVE: ["POTION", "ANTIDOTE"],
    });
    expect(phoneContacts.PHONE_MOM).toMatchObject({
      contactId: "PHONE_MOM",
      trainerLabel: "PHONECONTACT_MOM",
      mapConstant: "PLAYERS_HOUSE_1F",
      calleeTimeMask: 7,
      calleeScript: "MomPhoneCalleeScript",
    });
    expect(permanentPhoneNumbers).toEqual(["PHONE_MOM"]);
    expect(specialPhoneCalls).toEqual(["SPECIALCALL_NONE", "SPECIALCALL_MASTERBALL"]);
    expect(npcTrades).toEqual(["NPC_TRADE_MIKE", "NPC_TRADE_KYLE"]);
    expect(specialRoutines).toEqual(["FadeOutMusic", "HealParty"]);
    for (const fileName of [
      "flee_mons.json",
      "marts.json",
      "phone_contacts.json",
      "permanent_phone_numbers.json",
      "special_phone_calls.json",
      "npc_trades.json",
      "special_routines.json",
      "pc_strings.json",
      "menu_icons.json",
      "pokedex_entries.json",
      "pokemon_frontpic_anim.json",
    ]) {
      const targetPath = path.join(dataDir, fileName);
      expect(fs.existsSync(targetPath)).toBe(true);
      expect(fs.readFileSync(targetPath, "utf8").trim()).not.toBe("");
    }
  });

  it("fails instead of exporting empty flee tables when required labels are missing", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "flee_mons.asm"),
      ["AlwaysFleeMons:", "\tdb RAIKOU", "\tdb -1", ""].join("\n")
    );

    expect(() => exportFleeMons()).toThrow("Could not parse required OftenFleeMons table");
  });

  it("requires exact lowercase frontpic animation opcodes without case coercion", () => {
    writeFile(path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"), ["\tFRAME 1, 07", "\tendanim", ""].join("\n"));

    expect(() => exportRuntimeAssets()).toThrow("Unknown frontpic animation opcode 'FRAME'");
  });

  it("rejects malformed frontpic animation rows instead of skipping them", () => {
    writeFile(path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"), ["\tframe 1", "\tendanim", ""].join("\n"));

    expect(() => exportRuntimeAssets()).toThrow("Malformed frontpic animation frame row");
  });

  it("rejects unknown Pokedex entry file stems instead of normalizing them", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "pokemon", "dex_entries", "mr-mime.asm"),
      ['\tdb "BARRIER@"', "\tdw 403, 1200 ; height, weight", '\tdb "It mimics@"', ""].join("\n")
    );

    expect(() => exportRuntimeAssets()).toThrow("Unknown or case-changed runtime species file stem 'mr-mime'");
  });

  it("rejects unknown frontpic animation directories instead of normalizing them", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "mr-mime", "anim.asm"),
      ["\tframe 0, 05", "\tendanim", ""].join("\n")
    );

    expect(() => exportRuntimeAssets()).toThrow("Unknown or case-changed runtime species file stem 'mr-mime'");
  });

  it("requires permanent phone numbers to resolve to exact declared contacts or trainer labels", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "phone", "permanent_numbers.asm"), "\tdb PHONECONTACT_ELM\n\tdb -1\n");

    expect(() => exportPermanentPhoneNumbers()).toThrow(
      "Permanent phone number 'PHONECONTACT_ELM' does not match a declared phone contact id or trainer label"
    );
  });

  it("rejects lowercase hex phone sentinels instead of coercing them", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "phone", "permanent_numbers.asm"), "\tdb $ff\n");

    expect(() => exportPermanentPhoneNumbers()).toThrow(
      "Permanent phone number '$ff' does not match a declared phone contact id or trainer label"
    );
  });

  it("requires trainer phone contacts to have explicit exported class names", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "phone_constants.asm"),
      [
        "\tconst PHONE_00",
        "\tconst PHONE_MOM",
        "\tconst PHONE_JOEY",
        "\tDEF NUM_PHONE_CONTACTS EQU const_value",
        "; SpecialPhoneCallList indexes (see data/phone/special_calls.asm)",
        "\tconst_def",
        "\tconst SPECIALCALL_NONE",
        "\tDEF NUM_SPECIALCALLS EQU const_value - 1",
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "phone_contacts.asm"),
      [
        "\tphone TRAINER_NONE, 0, 0, 0, 0, 0, 0",
        "\tphone TRAINER_NONE, PHONECONTACT_MOM, PLAYERS_HOUSE_1F, ANYTIME, MomPhoneCalleeScript, 0, 0",
        "\tphone YOUNGSTER, JOEY1, ROUTE_30, ANYTIME, JoeyPhoneCalleeScript, ANYTIME, JoeyPhoneCallerScript",
        "",
      ].join("\n")
    );
    writeFile(path.join(mockDisassemblyRoot, "constants", "trainer_constants.asm"), ["trainerclass TRAINER_NONE", "trainerclass YOUNGSTER", ""].join("\n"));
    writeFile(path.join(mockDisassemblyRoot, "data", "trainers", "class_names.asm"), "");

    expect(() => exportRuntimeAssets()).toThrow(
      "Phone contact PHONE_JOEY references trainer class 'YOUNGSTER' without an exported class name"
    );
  });
});

describe("timeTokenToMask", () => {
  it("requires exact phone time-mask tokens without case coercion or unknown-token fallback", () => {
    expect(timeTokenToMask("ANYTIME")).toBe(7);
    expect(timeTokenToMask("MORN|NITE")).toBe(5);
    expect(timeTokenToMask("0")).toBe(0);
    expect(timeTokenToMask("8")).toBe(8);

    expect(() => timeTokenToMask("anytime")).toThrow("Unknown phone time mask token 'anytime'");
    expect(() => timeTokenToMask("MORN|late")).toThrow("Unknown phone time mask token 'late' in 'MORN|late'");
  });
});
