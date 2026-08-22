import fs from "fs";
import os from "os";
import path from "path";
import {
  exportBuenaPasswordCategories,
  exportBuenaPrizes,
  exportBattleStatMultipliers,
  exportBattleTowerRules,
  exportBugContestConfig,
  exportCaptureWobbleProbabilities,
  exportDratiniMoveSets,
  exportTypeCategories,
  exportTypeEffectivenessTable,
  exportWeatherModifiers,
  exportEncounterSlotTables,
  exportHappinessData,
  exportFleeMons,
  exportKurtApricornRecipes,
  exportMarts,
  exportMagikarpLengths,
  exportMenuIcons,
  exportMovePriorityTable,
  exportOakRatings,
  exportOddEggDefinitions,
  exportPcStrings,
  exportPhoneContacts,
  exportPermanentPhoneNumbers,
  exportRoamingPokemon,
  exportRuntimeAssets,
  exportShuckieGift,
  exportTrainerClassNames,
  timeTokenToMask,
} from "./export-runtime-assets";

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

const writeTypeConstantsFixture = (): void => {
  writeFile(
    path.join(mockDisassemblyRoot, "constants", "type_constants.asm"),
    [
      "\tconst_def",
      "DEF PHYSICAL EQU const_value",
      "\tconst NORMAL",
      "\tconst FIGHTING",
      "\tconst ROCK",
      "\tconst GHOST",
      "DEF UNUSED_TYPES EQU const_value",
      "\tconst_next 19",
      "\tconst CURSE_TYPE",
      "DEF UNUSED_TYPES_END EQU const_value",
      "DEF SPECIAL EQU const_value",
      "\tconst FIRE",
      "\tconst GRASS",
      "\tconst ELECTRIC",
      "\tconst GROUND",
      "DEF TYPES_END EQU const_value",
      "",
    ].join("\n"),
  );
};

const writeBattleStatMultiplierFixtures = (
  statRows: string[],
  accuracyRows: string[],
): void => {
  writeFile(
    path.join(mockDisassemblyRoot, "data", "battle", "stat_multipliers.asm"),
    statRows.join("\n"),
  );
  writeFile(
    path.join(
      mockDisassemblyRoot,
      "data",
      "battle",
      "accuracy_multipliers.asm",
    ),
    accuracyRows.join("\n"),
  );
};

const validBattleStatMultiplierRows = (): string[] =>
  Array.from({ length: 13 }, () => "\tdb 1, 1");

const canonicalBugContestEncounterRows = [
  "\tdb 20, CATERPIE,    7, 18",
  "\tdb 20, WEEDLE,      7, 18",
  "\tdb 10, METAPOD,     9, 18",
  "\tdb 10, KAKUNA,      9, 18",
  "\tdb  5, BUTTERFREE, 12, 15",
  "\tdb  5, BEEDRILL,   12, 15",
  "\tdb 10, VENONAT,    10, 16",
  "\tdb 10, PARAS,      10, 17",
  "\tdb  5, SCYTHER,    13, 14",
  "\tdb  5, PINSIR,     13, 14",
  "\tdb -1, VENOMOTH,   30, 40",
] as const;

const canonicalBugContestEncounters = [
  { weight: 20, species: "CATERPIE", minLevel: 7, maxLevel: 18 },
  { weight: 20, species: "WEEDLE", minLevel: 7, maxLevel: 18 },
  { weight: 10, species: "METAPOD", minLevel: 9, maxLevel: 18 },
  { weight: 10, species: "KAKUNA", minLevel: 9, maxLevel: 18 },
  { weight: 5, species: "BUTTERFREE", minLevel: 12, maxLevel: 15 },
  { weight: 5, species: "BEEDRILL", minLevel: 12, maxLevel: 15 },
  { weight: 10, species: "VENONAT", minLevel: 10, maxLevel: 16 },
  { weight: 10, species: "PARAS", minLevel: 10, maxLevel: 17 },
  { weight: 5, species: "SCYTHER", minLevel: 13, maxLevel: 14 },
  { weight: 5, species: "PINSIR", minLevel: 13, maxLevel: 14 },
  { weight: 255, species: "VENOMOTH", minLevel: 30, maxLevel: 40 },
] as const;

const writeBugContestConfigFixtures = ({
  timerSeconds = 0,
  contestantCount = 3,
  selectedContestantCount = 2,
  encounterSourceLines = ["ContestMons:", ...canonicalBugContestEncounterRows],
}: {
  timerSeconds?: number;
  contestantCount?: number;
  selectedContestantCount?: number;
  encounterSourceLines?: readonly string[];
} = {}): void => {
  writeFile(
    path.join(mockDisassemblyRoot, "constants", "script_constants.asm"),
    [
      "DEF BUG_CONTEST_BALLS EQU 20",
      "DEF BUG_CONTEST_MINUTES EQU 20",
      `DEF BUG_CONTEST_SECONDS EQU ${timerSeconds}`,
      `DEF NUM_BUG_CONTESTANTS EQU ${contestantCount}`,
      "",
    ].join("\n"),
  );
  writeFile(
    path.join(
      mockDisassemblyRoot,
      "engine",
      "events",
      "bug_contest",
      "contest_2.asm",
    ),
    [
      "SelectRandomBugContestContestants:",
      ".loop1",
      "\tld c, NUM_BUG_CONTESTANTS",
      `\tld c, ${selectedContestantCount}`,
      ".loop2",
      "CheckBugContestContestantFlag:",
      "",
    ].join("\n"),
  );
  writeFile(
    path.join(mockDisassemblyRoot, "data", "events", "bug_contest_flags.asm"),
    [
      "BugCatchingContestantEventFlagTable:",
      "\ttable_width 2",
      ...Array.from(
        { length: contestantCount },
        (_, index) => `\tdw EVENT_BUG_CATCHING_CONTESTANT_${index + 1}A`,
      ),
      "\tassert_table_length NUM_BUG_CONTESTANTS",
      "",
    ].join("\n"),
  );
  writeFile(
    path.join(mockDisassemblyRoot, "data", "wild", "bug_contest_mons.asm"),
    [...encounterSourceLines, ""].join("\n"),
  );
};

const canonicalRoamingRouteRows = [
  ["ROUTE_29", "ROUTE_30", "ROUTE_46"],
  ["ROUTE_30", "ROUTE_29", "ROUTE_31"],
  ["ROUTE_31", "ROUTE_30", "ROUTE_32", "ROUTE_36"],
  ["ROUTE_32", "ROUTE_36", "ROUTE_31", "ROUTE_33"],
  ["ROUTE_33", "ROUTE_32", "ROUTE_34"],
  ["ROUTE_34", "ROUTE_33", "ROUTE_35"],
  ["ROUTE_35", "ROUTE_34", "ROUTE_36"],
  ["ROUTE_36", "ROUTE_35", "ROUTE_31", "ROUTE_32", "ROUTE_37"],
  ["ROUTE_37", "ROUTE_36", "ROUTE_38", "ROUTE_42"],
  ["ROUTE_38", "ROUTE_37", "ROUTE_39", "ROUTE_42"],
  ["ROUTE_39", "ROUTE_38"],
  ["ROUTE_42", "ROUTE_43", "ROUTE_44", "ROUTE_37", "ROUTE_38"],
  ["ROUTE_43", "ROUTE_42", "ROUTE_44"],
  ["ROUTE_44", "ROUTE_42", "ROUTE_43", "ROUTE_45"],
  ["ROUTE_45", "ROUTE_44", "ROUTE_46"],
  ["ROUTE_46", "ROUTE_45", "ROUTE_29"],
] as const;

const roamingMapMetadata = {
  ROUTE_29: { groupId: 24, mapId: 3 },
  ROUTE_30: { groupId: 26, mapId: 1 },
  ROUTE_31: { groupId: 26, mapId: 2 },
  ROUTE_32: { groupId: 10, mapId: 1 },
  ROUTE_33: { groupId: 8, mapId: 6 },
  ROUTE_34: { groupId: 11, mapId: 1 },
  ROUTE_35: { groupId: 10, mapId: 2 },
  ROUTE_36: { groupId: 10, mapId: 3 },
  ROUTE_37: { groupId: 10, mapId: 4 },
  ROUTE_38: { groupId: 1, mapId: 12 },
  ROUTE_39: { groupId: 1, mapId: 13 },
  ROUTE_42: { groupId: 2, mapId: 5 },
  ROUTE_43: { groupId: 9, mapId: 5 },
  ROUTE_44: { groupId: 2, mapId: 6 },
  ROUTE_45: { groupId: 5, mapId: 8 },
  ROUTE_46: { groupId: 5, mapId: 9 },
};

const canonicalInitRoamMonsLines = [
  "InitRoamMons:",
  "\tld a, RAIKOU",
  "\tld [wRoamMon1Species], a",
  "\tld a, ENTEI",
  "\tld [wRoamMon2Species], a",
  "\tld a, 40",
  "\tld [wRoamMon1Level], a",
  "\tld [wRoamMon2Level], a",
  "\tld a, GROUP_ROUTE_42",
  "\tld [wRoamMon1MapGroup], a",
  "\tld a, MAP_ROUTE_42",
  "\tld [wRoamMon1MapNumber], a",
  "\tld a, GROUP_ROUTE_37",
  "\tld [wRoamMon2MapGroup], a",
  "\tld a, MAP_ROUTE_37",
  "\tld [wRoamMon2MapNumber], a",
  "\txor a",
  "\tld [wRoamMon1HP], a",
  "\tld [wRoamMon2HP], a",
  "\tret",
  "CheckEncounterRoamMon:",
] as const;

const writeRoamingCatalogFixtures = ({
  initLines = [...canonicalInitRoamMonsLines],
  routeRows = canonicalRoamingRouteRows.map((row) => [...row]),
  roamMapCount = 16,
  maskOperand = "NUM_ROAMMON_MAPS",
  inactiveGroup = -1,
  inactiveNumber = -1,
}: {
  initLines?: string[];
  routeRows?: string[][];
  roamMapCount?: number;
  maskOperand?: string;
  inactiveGroup?: number;
  inactiveNumber?: number;
} = {}): void => {
  writeFile(
    path.join(mockDisassemblyRoot, "engine", "overworld", "wildmons.asm"),
    [
      ...initLines,
      "JumpRoamMon:",
      "\tcall Random",
      `\tmaskbits ${maskOperand}`,
      "\tcp NUM_ROAMMON_MAPS",
      "",
    ].join("\n"),
  );
  writeFile(
    path.join(mockDisassemblyRoot, "data", "wild", "roammon_maps.asm"),
    [
      "RoamMaps:",
      ...routeRows.map((row) => `\troam_map ${row.join(", ")}`),
      "\tdb -1 ; end",
      "",
    ].join("\n"),
  );
  writeFile(
    path.join(mockDisassemblyRoot, "constants", "pokemon_data_constants.asm"),
    `DEF NUM_ROAMMON_MAPS EQU ${roamMapCount}\n`,
  );
  writeFile(
    path.join(mockDisassemblyRoot, "constants", "map_data_constants.asm"),
    [
      `DEF GROUP_N_A EQU ${inactiveGroup}`,
      `DEF MAP_N_A EQU ${inactiveNumber}`,
      "DEF GROUP_NONE EQU 0",
      "DEF MAP_NONE EQU 0",
      "",
    ].join("\n"),
  );
};

const writeMinimalHappinessFixtures = (probabilityRows: string[]): void => {
  writeFile(
    path.join(mockDisassemblyRoot, "constants", "pokemon_data_constants.asm"),
    [
      "; ChangeHappiness arguments (see data/events/happiness_changes.asm)",
      "\tconst_def 1",
      "\tconst HAPPINESS_OLDERCUT1",
      "\tconst HAPPINESS_YOUNGCUT1",
      "\tconst HAPPINESS_GROOMING",
      "\tDEF NUM_HAPPINESS_CHANGES EQU const_value - 1",
      "",
    ].join("\n"),
  );
  writeFile(
    path.join(mockDisassemblyRoot, "data", "events", "happiness_changes.asm"),
    [
      "HappinessChanges:",
      "\ttable_width 3, HappinessChanges",
      "\tdb +1, +1, +1",
      "\tdb +2, +2, +2",
      "\tdb +3, +3, +1",
      "\tassert_table_length NUM_HAPPINESS_CHANGES",
      "",
    ].join("\n"),
  );
  writeFile(
    path.join(
      mockDisassemblyRoot,
      "data",
      "events",
      "happiness_probabilities.asm",
    ),
    probabilityRows.join("\n"),
  );
};

describe("exportRuntimeAssets", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(
      path.join(os.tmpdir(), "pokecrystal-runtime-export-"),
    );
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
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "marts.asm"),
      [
        "MartCherrygrove:",
        "\tdb 2",
        "\tdb POTION",
        "\tdb ANTIDOTE",
        "\tdb -1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "item_constants.asm"),
      "",
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "pokemon_constants.asm"),
      [
        "\tconst_def 1",
        "\tconst CHIKORITA",
        "\tconst MR__MIME",
        "\tconst UNOWN_A",
        "\tconst EGG",
        "",
      ].join("\n"),
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
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "npc_trade_constants.asm"),
      ["\tconst NPC_TRADE_MIKE", "\tconst NPC_TRADE_KYLE", ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "npc_trades.asm"),
      [
        'npctrade TRADE_DIALOGSET_COLLECTOR, ABRA, MACHOP, "MUSCLE", $37, $66, GOLD_BERRY, 37460, "MIKE", TRADE_GENDER_EITHER',
        'npctrade TRADE_DIALOGSET_COLLECTOR, BELLSPROUT, ONIX, "ROCKY", $96, $66, BITTER_BERRY, 48926, "KYLE", TRADE_GENDER_EITHER',
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "phone_contacts.asm"),
      [
        "\tphone TRAINER_NONE, 0, 0, 0, 0, 0, 0",
        "\tphone TRAINER_NONE, PHONECONTACT_MOM, PLAYERS_HOUSE_1F, ANYTIME, MomPhoneCalleeScript, 0, 0",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "permanent_numbers.asm"),
      "\tdb PHONE_MOM\n\tdb -1\n",
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "special_calls.asm"),
      [
        "SpecialPhoneCallList:",
        "\tspecialcall SpecialCallOnlyWhenOutside, PHONECONTACT_MOM, MomPhoneCallerScript",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "non_trainer_names.asm"),
      '.mom: db "MOM:@"\n',
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "trainer_constants.asm"),
      "trainerclass TRAINER_NONE\ntrainerclass YOUNGSTER\n",
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "trainers", "sprites.asm"),
      "db SPRITE_YOUNGSTER\n",
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "trainers", "genders.asm"),
      "db MALE ; YOUNGSTER\n",
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "battle_tower", "classes.asm"),
      'bt_trainer YOUNGSTER, "JOEY"\n',
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "battle_tower", "parties.asm"),
      [
        "; BattleTowerMons group 1",
        "db CHIKORITA",
        "db NO_ITEM",
        "db TACKLE, NO_MOVE, NO_MOVE, NO_MOVE",
        "dw 1234",
        "bigdt 1000",
        "bigdw 0",
        "bigdw 0",
        "bigdw 0",
        "bigdw 0",
        "bigdw 0",
        "dn 8, 8, 8, 8",
        "db 35, 0, 0, 0",
        "db 70",
        "db 0, 0",
        "db 10",
        "db 0, 0",
        "bigdw 30",
        "bigdw 30",
        "bigdw 20",
        "bigdw 20",
        "bigdw 20",
        "bigdw 20",
        "bigdw 20",
        'dname "CHIKORITA"',
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "trainers", "class_names.asm"),
      'li "YOUNGSTER"\n',
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "special_pointers.asm"),
      [
        "SpecialsPointers:",
        "\tadd_special FadeOutMusic",
        "\tadd_special HealParty",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "pokemon", "bills_pc.asm"),
      'PCString_ChooseaPKMN: db "Choose a <PK><MN>.@"\n',
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "pokemon", "menu_icons.asm"),
      [
        "\tdb ICON_CHIKORITA ; CHIKORITA",
        "\tdb ICON_HUMANSHAPE ; MR__MIME",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "pokemon",
        "dex_entries",
        "chikorita.asm",
      ),
      [
        '\tdb "LEAF@"',
        "\tdw 211, 140 ; height, weight",
        '\tdb "A sweet aroma"',
        '\tnext "gently wafts@"',
        '\tpage "from the leaf"',
        '\tnext "on its head.@"',
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "pokemon",
        "dex_entries",
        "mr__mime.asm",
      ),
      [
        '\tdb "BARRIER@"',
        "\tdw 403, 1200 ; height, weight",
        '\tdb "It mimics@"',
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      [
        "\tframe 1, 07",
        "\tsetrepeat 2",
        "\tframe 0, 05",
        "\tdorepeat 1",
        "\tendanim",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "mr__mime", "anim.asm"),
      ["\tframe 0, 05", "\tendanim", ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "unown_a", "anim.asm"),
      ["\tframe 0, 05", "\tendanim", ""].join("\n"),
    );
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports required runtime JSON assets including flee_mons.json", () => {
    exportRuntimeAssets();

    const dataDir = path.join(mockAssetsRoot, "data");
    const fleeMons = JSON.parse(
      fs.readFileSync(path.join(dataDir, "flee_mons.json"), "utf8"),
    );
    const pokedexEntries = JSON.parse(
      fs.readFileSync(path.join(dataDir, "pokedex_entries.json"), "utf8"),
    );
    const frontpicAnimations = JSON.parse(
      fs.readFileSync(path.join(dataDir, "pokemon_frontpic_anim.json"), "utf8"),
    );
    const phoneContacts = JSON.parse(
      fs.readFileSync(path.join(dataDir, "phone_contacts.json"), "utf8"),
    );
    const permanentPhoneNumbers = JSON.parse(
      fs.readFileSync(
        path.join(dataDir, "permanent_phone_numbers.json"),
        "utf8",
      ),
    );
    const specialPhoneCalls = JSON.parse(
      fs.readFileSync(path.join(dataDir, "special_phone_calls.json"), "utf8"),
    );
    const npcTrades = JSON.parse(
      fs.readFileSync(path.join(dataDir, "npc_trades.json"), "utf8"),
    );
    const specialRoutines = JSON.parse(
      fs.readFileSync(path.join(dataDir, "special_routines.json"), "utf8"),
    );

    expect(fleeMons).toEqual({
      buckets: {
        always: ["RAIKOU", "ENTEI"],
        often: ["DELIBIRD"],
        sometimes: ["MAGNEMITE"],
      },
    });
    expect(pokedexEntries.CHIKORITA).toMatchObject({
      species: "CHIKORITA",
      classification: "LEAF",
      pages: ["A sweet aroma @ gently wafts", "from the leaf @ on its head."],
    });
    expect(pokedexEntries.MR__MIME).toEqual(
      expect.objectContaining({
        species: "MR__MIME",
        classification: "BARRIER",
      }),
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
    const menuIcons = JSON.parse(
      fs.readFileSync(path.join(dataDir, "menu_icons.json"), "utf8"),
    );
    expect(menuIcons.MR__MIME).toBe("ICON_HUMANSHAPE");
    const marts = JSON.parse(
      fs.readFileSync(path.join(dataDir, "marts.json"), "utf8"),
    );
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
    expect(permanentPhoneNumbers).toEqual({
      PHONE_MOM: { listIndex: 0 },
    });
    expect(specialPhoneCalls).toEqual({
      SPECIALCALL_MASTERBALL: {
        value: 1,
        condition: "SpecialCallOnlyWhenOutside",
        contactId: "PHONE_MOM",
        callerScript: "MomPhoneCallerScript",
      },
    });
    expect(npcTrades).toEqual({
      NPC_TRADE_MIKE: {
        dialogSet: "TRADE_DIALOGSET_COLLECTOR",
        requestedSpecies: "ABRA",
        offeredSpecies: "MACHOP",
        nickname: "MUSCLE",
        dvs: [55, 102],
        heldItem: "GOLD_BERRY",
        originalTrainerId: 37460,
        originalTrainerName: "MIKE",
        genderRequirement: "TRADE_GENDER_EITHER",
      },
      NPC_TRADE_KYLE: {
        dialogSet: "TRADE_DIALOGSET_COLLECTOR",
        requestedSpecies: "BELLSPROUT",
        offeredSpecies: "ONIX",
        nickname: "ROCKY",
        dvs: [150, 102],
        heldItem: "BITTER_BERRY",
        originalTrainerId: 48926,
        originalTrainerName: "KYLE",
        genderRequirement: "TRADE_GENDER_EITHER",
      },
    });
    expect(specialRoutines).toEqual({
      FadeOutMusic: {},
      HealParty: {},
    });
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
      ["AlwaysFleeMons:", "\tdb RAIKOU", "\tdb -1", ""].join("\n"),
    );

    expect(() => exportFleeMons()).toThrow(
      "Could not parse required OftenFleeMons table",
    );
  });

  it("rejects duplicate mart labels before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "marts.asm"),
      [
        "MartCherrygrove:",
        "\tdb 1",
        "\tdb POTION",
        "\tdb -1",
        "MartCherrygrove:",
        "\tdb 1",
        "\tdb ANTIDOTE",
        "\tdb -1",
        "",
      ].join("\n"),
    );

    expect(() => exportMarts()).toThrow(
      "Duplicate mart table 'MART_CHERRYGROVE'.",
    );
  });

  it("rejects duplicate PC string labels before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "pokemon", "bills_pc.asm"),
      [
        'PCString_ChooseaPKMN: db "Choose a <PK><MN>.@"',
        'PCString_ChooseaPKMN: db "Pick one.@"',
        "",
      ].join("\n"),
    );

    expect(() => exportPcStrings()).toThrow(
      "Duplicate PC string 'PCString_ChooseaPKMN'.",
    );
  });

  it("rejects empty PC strings before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "pokemon", "bills_pc.asm"),
      'PCString_ChooseaPKMN: db "@"\n',
    );

    expect(() => exportPcStrings()).toThrow(
      "PC string 'PCString_ChooseaPKMN' must be nonempty.",
    );
  });

  it("rejects duplicate menu icon species before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "pokemon", "menu_icons.asm"),
      [
        "\tdb ICON_CHIKORITA ; CHIKORITA",
        "\tdb ICON_HUMANSHAPE ; CHIKORITA",
        "",
      ].join("\n"),
    );

    expect(() => exportMenuIcons()).toThrow(
      "Duplicate menu icon species 'CHIKORITA'.",
    );
  });

  it("rejects source-declared EGG menu icons before built-in insertion", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "pokemon", "menu_icons.asm"),
      ["\tdb ICON_EGG ; EGG", ""].join("\n"),
    );

    expect(() => exportMenuIcons()).toThrow(
      "Menu icon table must not declare built-in EGG icon.",
    );
  });

  it("exports one source-exact roaming catalog with partial ordered init writes and the full numeric graph", () => {
    writeRoamingCatalogFixtures();

    const roamers = exportRoamingPokemon(roamingMapMetadata);

    expect(Object.keys(roamers)).toEqual([
      "slotCount",
      "inactiveMap",
      "initWrites",
      "routes",
      "jumpMask",
    ]);
    expect(roamers).toEqual({
      slotCount: 3,
      inactiveMap: { mapGroup: 255, mapNumber: 255 },
      initWrites: [
        {
          slot: 0,
          species: "RAIKOU",
          level: 40,
          mapGroup: 2,
          mapNumber: 5,
          hp: 0,
        },
        {
          slot: 1,
          species: "ENTEI",
          level: 40,
          mapGroup: 10,
          mapNumber: 4,
          hp: 0,
        },
      ],
      routes: [
        {
          mapGroup: 24,
          mapNumber: 3,
          connections: [
            { mapGroup: 26, mapNumber: 1 },
            { mapGroup: 5, mapNumber: 9 },
          ],
        },
        {
          mapGroup: 26,
          mapNumber: 1,
          connections: [
            { mapGroup: 24, mapNumber: 3 },
            { mapGroup: 26, mapNumber: 2 },
          ],
        },
        {
          mapGroup: 26,
          mapNumber: 2,
          connections: [
            { mapGroup: 26, mapNumber: 1 },
            { mapGroup: 10, mapNumber: 1 },
            { mapGroup: 10, mapNumber: 3 },
          ],
        },
        {
          mapGroup: 10,
          mapNumber: 1,
          connections: [
            { mapGroup: 10, mapNumber: 3 },
            { mapGroup: 26, mapNumber: 2 },
            { mapGroup: 8, mapNumber: 6 },
          ],
        },
        {
          mapGroup: 8,
          mapNumber: 6,
          connections: [
            { mapGroup: 10, mapNumber: 1 },
            { mapGroup: 11, mapNumber: 1 },
          ],
        },
        {
          mapGroup: 11,
          mapNumber: 1,
          connections: [
            { mapGroup: 8, mapNumber: 6 },
            { mapGroup: 10, mapNumber: 2 },
          ],
        },
        {
          mapGroup: 10,
          mapNumber: 2,
          connections: [
            { mapGroup: 11, mapNumber: 1 },
            { mapGroup: 10, mapNumber: 3 },
          ],
        },
        {
          mapGroup: 10,
          mapNumber: 3,
          connections: [
            { mapGroup: 10, mapNumber: 2 },
            { mapGroup: 26, mapNumber: 2 },
            { mapGroup: 10, mapNumber: 1 },
            { mapGroup: 10, mapNumber: 4 },
          ],
        },
        {
          mapGroup: 10,
          mapNumber: 4,
          connections: [
            { mapGroup: 10, mapNumber: 3 },
            { mapGroup: 1, mapNumber: 12 },
            { mapGroup: 2, mapNumber: 5 },
          ],
        },
        {
          mapGroup: 1,
          mapNumber: 12,
          connections: [
            { mapGroup: 10, mapNumber: 4 },
            { mapGroup: 1, mapNumber: 13 },
            { mapGroup: 2, mapNumber: 5 },
          ],
        },
        {
          mapGroup: 1,
          mapNumber: 13,
          connections: [{ mapGroup: 1, mapNumber: 12 }],
        },
        {
          mapGroup: 2,
          mapNumber: 5,
          connections: [
            { mapGroup: 9, mapNumber: 5 },
            { mapGroup: 2, mapNumber: 6 },
            { mapGroup: 10, mapNumber: 4 },
            { mapGroup: 1, mapNumber: 12 },
          ],
        },
        {
          mapGroup: 9,
          mapNumber: 5,
          connections: [
            { mapGroup: 2, mapNumber: 5 },
            { mapGroup: 2, mapNumber: 6 },
          ],
        },
        {
          mapGroup: 2,
          mapNumber: 6,
          connections: [
            { mapGroup: 2, mapNumber: 5 },
            { mapGroup: 9, mapNumber: 5 },
            { mapGroup: 5, mapNumber: 8 },
          ],
        },
        {
          mapGroup: 5,
          mapNumber: 8,
          connections: [
            { mapGroup: 2, mapNumber: 6 },
            { mapGroup: 5, mapNumber: 9 },
          ],
        },
        {
          mapGroup: 5,
          mapNumber: 9,
          connections: [
            { mapGroup: 5, mapNumber: 8 },
            { mapGroup: 24, mapNumber: 3 },
          ],
        },
      ],
      jumpMask: 15,
    });
    expect(roamers.initWrites).toHaveLength(2);
    expect(roamers.initWrites.every((write) => !("dvs" in write))).toBe(true);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "roaming_pokemon.json"), "utf8"),
      ),
    ).toEqual(roamers);
  });

  it("rejects reordered and extra InitRoamMons field writes", () => {
    const reordered: string[] = [...canonicalInitRoamMonsLines];
    [reordered[2], reordered[4]] = [reordered[4], reordered[2]];
    writeRoamingCatalogFixtures({ initLines: reordered });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "InitRoamMons field write 0",
    );

    const withExtraWrite: string[] = [...canonicalInitRoamMonsLines];
    withExtraWrite.splice(-2, 0, "\tld [wRoamMon3HP], a");
    writeRoamingCatalogFixtures({ initLines: withExtraWrite });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "InitRoamMons has 11 roaming field writes, expected exactly 10.",
    );
  });

  it("rejects missing, duplicate, reordered, and unknown roaming route rows", () => {
    const missing = canonicalRoamingRouteRows
      .slice(0, -1)
      .map((row) => [...row]);
    writeRoamingCatalogFixtures({ routeRows: missing });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "RoamMaps has 15 rows, expected exactly 16.",
    );

    const duplicate = canonicalRoamingRouteRows.map((row) => [...row]);
    duplicate[1][0] = duplicate[0][0];
    writeRoamingCatalogFixtures({ routeRows: duplicate });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "RoamMaps repeats origin 'ROUTE_29'.",
    );

    const reordered = canonicalRoamingRouteRows.map((row) => [...row]);
    [reordered[0], reordered[1]] = [reordered[1], reordered[0]];
    writeRoamingCatalogFixtures({ routeRows: reordered });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "RoamMaps row 0 origin 'ROUTE_30' does not match canonical 'ROUTE_29'.",
    );

    const unknown: string[][] = canonicalRoamingRouteRows.map((row) => [
      ...row,
    ]);
    unknown[0][1] = "ROUTE_99";
    writeRoamingCatalogFixtures({ routeRows: unknown });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "RoamMaps row 0 connections",
    );
  });

  it("rejects bad roaming connection counts, jump masks, and map metadata", () => {
    const badConnections = canonicalRoamingRouteRows.map((row) => [...row]);
    badConnections[0] = [
      "ROUTE_29",
      "ROUTE_30",
      "ROUTE_31",
      "ROUTE_32",
      "ROUTE_33",
      "ROUTE_34",
    ];
    writeRoamingCatalogFixtures({ routeRows: badConnections });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "RoamMaps row 0 must declare 1..4 connections, found 5.",
    );

    writeRoamingCatalogFixtures({ maskOperand: "15" });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "JumpRoamMon must mask with exact NUM_ROAMMON_MAPS.",
    );

    writeRoamingCatalogFixtures();
    const missingMetadata = { ...roamingMapMetadata } as Record<
      string,
      { groupId: number; mapId: number }
    >;
    delete missingMetadata.ROUTE_46;
    expect(() => exportRoamingPokemon(missingMetadata)).toThrow(
      "Roaming map 'ROUTE_46' is missing from runtime map metadata.",
    );

    const inactiveGroupCollision = {
      ...roamingMapMetadata,
      UNUSED_RUNTIME_MAP: { groupId: 0xff, mapId: 1 },
    };
    expect(() => exportRoamingPokemon(inactiveGroupCollision)).toThrow(
      "Inactive roaming map group 255 collides with runtime map 'UNUSED_RUNTIME_MAP'.",
    );
  });

  it("requires the canonical 16-row roaming count before deriving jumpMask 15", () => {
    writeRoamingCatalogFixtures({ roamMapCount: 8 });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "NUM_ROAMMON_MAPS is 8, expected exact source count 16.",
    );
  });

  it("parses the retired roaming map sentinel and rejects source mutation away from -1/-1", () => {
    writeRoamingCatalogFixtures({ inactiveGroup: -2 });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "GROUP_N_A is -2, expected exact inactive sentinel -1.",
    );

    writeRoamingCatalogFixtures({ inactiveNumber: 0 });
    expect(() => exportRoamingPokemon(roamingMapMetadata)).toThrow(
      "MAP_N_A is 0, expected exact inactive sentinel -1.",
    );
  });

  it("exports Buena prize definitions from exact ASM item/cost rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "buena_prizes.asm"),
      [
        "BuenaPrizeItems:",
        "\ttable_width 2",
        "\tdb ULTRA_BALL,   2",
        "\tdb RARE_CANDY,   3",
        "\tassert_table_length NUM_BUENA_PRIZES",
        "",
      ].join("\n"),
    );

    const prizes = exportBuenaPrizes();

    expect(prizes).toEqual({
      ULTRA_BALL: 2,
      RARE_CANDY: 3,
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "buena_prizes.json"), "utf8"),
      ),
    ).toEqual(prizes);
  });

  it("rejects duplicate Buena prize items before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "buena_prizes.asm"),
      [
        "BuenaPrizeItems:",
        "\ttable_width 2",
        "\tdb RARE_CANDY,   3",
        "\tdb RARE_CANDY,   5",
        "\tassert_table_length NUM_BUENA_PRIZES",
        "",
      ].join("\n"),
    );

    expect(() => exportBuenaPrizes()).toThrow(
      "Duplicate Buena prize item 'RARE_CANDY'.",
    );
  });

  it("rejects Buena prize costs outside byte range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "buena_prizes.asm"),
      [
        "BuenaPrizeItems:",
        "\ttable_width 2",
        "\tdb RARE_CANDY, 256",
        "\tassert_table_length NUM_BUENA_PRIZES",
        "",
      ].join("\n"),
    );

    expect(() => exportBuenaPrizes()).toThrow(
      "Buena prize cost 256 is outside byte range.",
    );
  });

  it("exports Buena password categories from exact ASM table order and rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "radio", "buenas_passwords.asm"),
      [
        "BuenasPasswordTable:",
        "\ttable_width 2",
        "\tdw .HealingItems",
        "\tdw .RadioStations",
        "\tassert_table_length NUM_PASSWORD_CATEGORIES",
        ".HealingItems:  db BUENA_ITEM,   12, POTION, ANTIDOTE, PARLYZ_HEAL",
        '.RadioStations: db BUENA_STRING, 13, "#MON Talk@", "#MON Music@", "Lucky Channel@"',
        "",
      ].join("\n"),
    );

    const categories = exportBuenaPasswordCategories();

    expect(categories).toEqual({
      order: ["HealingItems", "RadioStations"],
      categories: {
        HealingItems: {
          categoryType: "BUENA_ITEM",
          points: 12,
          options: ["POTION", "ANTIDOTE", "PARLYZ_HEAL"],
        },
        RadioStations: {
          categoryType: "BUENA_STRING",
          points: 13,
          options: ["#MON Talk", "#MON Music", "Lucky Channel"],
        },
      },
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(
          path.join(dataDir, "buena_password_categories.json"),
          "utf8",
        ),
      ),
    ).toEqual(categories);
  });

  it("rejects duplicate Buena password category rows before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "radio", "buenas_passwords.asm"),
      [
        "BuenasPasswordTable:",
        "\ttable_width 2",
        "\tdw .HealingItems",
        "\tassert_table_length NUM_PASSWORD_CATEGORIES",
        ".HealingItems: db BUENA_ITEM, 12, POTION, ANTIDOTE, PARLYZ_HEAL",
        ".HealingItems: db BUENA_ITEM, 13, POTION, ANTIDOTE, PARLYZ_HEAL",
        "",
      ].join("\n"),
    );

    expect(() => exportBuenaPasswordCategories()).toThrow(
      "Duplicate Buena password category 'HealingItems'.",
    );
  });

  it("rejects Buena password category points outside byte range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "radio", "buenas_passwords.asm"),
      [
        "BuenasPasswordTable:",
        "\ttable_width 2",
        "\tdw .HealingItems",
        "\tassert_table_length NUM_PASSWORD_CATEGORIES",
        ".HealingItems: db BUENA_ITEM, 256, POTION, ANTIDOTE, PARLYZ_HEAL",
        "",
      ].join("\n"),
    );

    expect(() => exportBuenaPasswordCategories()).toThrow(
      "Buena password category HealingItems points 256 is outside byte range.",
    );
  });

  it("exports Kurt apricorn recipes from exact ASM apricorn/ball rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "apricorn_balls.asm"),
      [
        "ApricornBalls:",
        "\tdb RED_APRICORN, LEVEL_BALL",
        "\tdb BLU_APRICORN, LURE_BALL",
        "\tdb -1",
        "",
      ].join("\n"),
    );

    const recipes = exportKurtApricornRecipes();

    expect(recipes).toEqual({
      RED_APRICORN: "LEVEL_BALL",
      BLU_APRICORN: "LURE_BALL",
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(
          path.join(dataDir, "kurt_apricorn_recipes.json"),
          "utf8",
        ),
      ),
    ).toEqual(recipes);
  });

  it("rejects duplicate Kurt apricorn recipes before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "apricorn_balls.asm"),
      [
        "ApricornBalls:",
        "\tdb RED_APRICORN, LEVEL_BALL",
        "\tdb RED_APRICORN, FAST_BALL",
        "\tdb -1",
        "",
      ].join("\n"),
    );

    expect(() => exportKurtApricornRecipes()).toThrow(
      "Duplicate Kurt apricorn recipe 'RED_APRICORN'.",
    );
  });

  it("exports Shuckie gift data from exact ASM labels and stores decimal OT id", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "shuckle.asm"),
      [
        "DEF MANIA_OT_ID EQU 00518",
        "",
        "GiveShuckle:",
        "\tld a, SHUCKLE",
        "\tld [wCurPartySpecies], a",
        "\tld a, 15",
        "\tld [wCurPartyLevel], a",
        "\tld [hl], BERRY",
        "\tset DAILYFLAGS1_GOT_SHUCKIE_TODAY_F, [hl]",
        "",
        "SpecialShuckleOT:",
        '\tdb "MANIA@"',
        "",
        "SpecialShuckleNickname:",
        '\tdb "SHUCKIE@"',
        "",
        "ReturnShuckie:",
        "",
      ].join("\n"),
    );

    const gift = exportShuckieGift();

    expect(gift).toEqual({
      species: "SHUCKLE",
      level: 15,
      heldItem: "BERRY",
      nickname: "SHUCKIE",
      originalTrainerName: "MANIA",
      originalTrainerId: 518,
      gotTodayEngineFlag: "ENGINE_GOT_SHUCKIE_TODAY",
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "shuckie_gift.json"), "utf8"),
      ),
    ).toEqual(gift);
  });

  it("rejects Shuckie gift levels outside Pokemon range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "shuckle.asm"),
      [
        "DEF MANIA_OT_ID EQU 00518",
        "GiveShuckle:",
        "\tld a, SHUCKLE",
        "\tld [wCurPartySpecies], a",
        "\tld a, 101",
        "\tld [wCurPartyLevel], a",
        "\tld [hl], BERRY",
        "\tset DAILYFLAGS1_GOT_SHUCKIE_TODAY_F, [hl]",
        "SpecialShuckleOT:",
        '\tdb "MANIA@"',
        "SpecialShuckleNickname:",
        '\tdb "SHUCKIE@"',
        "ReturnShuckie:",
        "",
      ].join("\n"),
    );

    expect(() => exportShuckieGift()).toThrow(
      "Shuckie gift level 101 is outside Pokemon level range.",
    );
  });

  it("rejects Shuckie original trainer ids outside word range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "shuckle.asm"),
      [
        "DEF MANIA_OT_ID EQU 65536",
        "GiveShuckle:",
        "\tld a, SHUCKLE",
        "\tld [wCurPartySpecies], a",
        "\tld a, 15",
        "\tld [wCurPartyLevel], a",
        "\tld [hl], BERRY",
        "\tset DAILYFLAGS1_GOT_SHUCKIE_TODAY_F, [hl]",
        "SpecialShuckleOT:",
        '\tdb "MANIA@"',
        "SpecialShuckleNickname:",
        '\tdb "SHUCKIE@"',
        "ReturnShuckie:",
        "",
      ].join("\n"),
    );

    expect(() => exportShuckieGift()).toThrow(
      "Shuckie gift original trainer id 65536 is outside word range.",
    );
  });

  it("exports Dratini move sets from exact zero-terminated ASM movesets", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "dratini.asm"),
      [
        ".Movesets:",
        ".Moveset0:",
        "\tdb WRAP",
        "\tdb THUNDER_WAVE",
        "\tdb TWISTER",
        "\tdb EXTREMESPEED",
        "\tdb 0",
        ".Moveset1:",
        "\tdb WRAP",
        "\tdb LEER",
        "\tdb THUNDER_WAVE",
        "\tdb TWISTER",
        "\tdb 0",
        "",
      ].join("\n"),
    );

    const moveSets = exportDratiniMoveSets();

    expect(moveSets).toEqual({
      "0": ["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"],
      "1": ["WRAP", "LEER", "THUNDER_WAVE", "TWISTER"],
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "dratini_move_sets.json"), "utf8"),
      ),
    ).toEqual(moveSets);
  });

  it("rejects duplicate Dratini moveset modes before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "dratini.asm"),
      [
        ".Movesets:",
        ".Moveset0:",
        "\tdb WRAP",
        "\tdb 0",
        ".Moveset0:",
        "\tdb LEER",
        "\tdb 0",
        "",
      ].join("\n"),
    );

    expect(() => exportDratiniMoveSets()).toThrow(
      "Duplicate Dratini moveset 0.",
    );
  });

  it("rejects empty Dratini movesets before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "dratini.asm"),
      [".Movesets:", ".Moveset0:", "\tdb 0", ""].join("\n"),
    );

    expect(() => exportDratiniMoveSets()).toThrow(
      "Dratini moveset 0 must not be empty.",
    );
  });

  it("rejects Dratini movesets exceeding party move limit before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "dratini.asm"),
      [
        ".Movesets:",
        ".Moveset0:",
        "\tdb WRAP",
        "\tdb LEER",
        "\tdb THUNDER_WAVE",
        "\tdb TWISTER",
        "\tdb EXTREMESPEED",
        "\tdb 0",
        "",
      ].join("\n"),
    );

    expect(() => exportDratiniMoveSets()).toThrow(
      "Dratini moveset 0 has 5 moves, exceeding party move limit.",
    );
  });

  it("exports Bug-Catching Contest config from exact ASM constants and flag table", () => {
    writeBugContestConfigFixtures();

    const config = exportBugContestConfig();

    expect(config).toEqual({
      parkBalls: 20,
      timerMinutes: 20,
      timerSeconds: 0,
      selectedContestantCount: 2,
      contestantFlags: [
        "EVENT_BUG_CATCHING_CONTESTANT_1A",
        "EVENT_BUG_CATCHING_CONTESTANT_2A",
        "EVENT_BUG_CATCHING_CONTESTANT_3A",
      ],
      encounters: canonicalBugContestEncounters,
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "bug_contest_config.json"), "utf8"),
      ),
    ).toEqual(config);
  });

  it("rejects Bug-Catching Contest timer seconds outside clock range before pack emission", () => {
    writeBugContestConfigFixtures({ timerSeconds: 60 });

    expect(() => exportBugContestConfig()).toThrow(
      "Bug-Catching Contest timer seconds 60 is outside clock second range.",
    );
  });

  it("rejects Bug-Catching Contest selected count above contestant flags before pack emission", () => {
    writeBugContestConfigFixtures({
      contestantCount: 2,
      selectedContestantCount: 3,
    });

    expect(() => exportBugContestConfig()).toThrow(
      "Bug-Catching Contest selected contestant count 3 exceeds contestant flags 2.",
    );
  });

  it("rejects a missing required Bug-Catching Contest encounter source", () => {
    writeBugContestConfigFixtures();
    fs.rmSync(
      path.join(mockDisassemblyRoot, "data", "wild", "bug_contest_mons.asm"),
    );

    expect(() => exportBugContestConfig()).toThrow(
      "Required Bug-Catching Contest encounter source data/wild/bug_contest_mons.asm could not be read.",
    );
  });

  it.each([
    {
      name: "missing ContestMons label",
      lines: [...canonicalBugContestEncounterRows],
      message:
        "Bug-Catching Contest encounter source must begin with exact ContestMons: label.",
    },
    {
      name: "malformed row opcode",
      lines: [
        "ContestMons:",
        canonicalBugContestEncounterRows[0].replace("db ", "dw "),
        ...canonicalBugContestEncounterRows.slice(1),
      ],
      message: "Malformed Bug-Catching Contest encounter row 1:",
    },
    {
      name: "reordered canonical rows",
      lines: [
        "ContestMons:",
        canonicalBugContestEncounterRows[1],
        canonicalBugContestEncounterRows[0],
        ...canonicalBugContestEncounterRows.slice(2),
      ],
      message:
        "Bug-Catching Contest encounter row 1 changed canonical order or values",
    },
    {
      name: "altered canonical row values",
      lines: [
        "ContestMons:",
        canonicalBugContestEncounterRows[0].replace("20,", "19,"),
        ...canonicalBugContestEncounterRows.slice(1),
      ],
      message:
        "Bug-Catching Contest encounter row 1 changed canonical order or values",
    },
    {
      name: "missing terminator row",
      lines: ["ContestMons:", ...canonicalBugContestEncounterRows.slice(0, -1)],
      message:
        "Bug-Catching Contest encounter table must contain exactly 10 weighted rows and one final sentinel row; found 10 rows.",
    },
    {
      name: "early terminator row",
      lines: [
        "ContestMons:",
        canonicalBugContestEncounterRows.at(-1)!,
        ...canonicalBugContestEncounterRows.slice(1, -1),
        canonicalBugContestEncounterRows[0],
      ],
      message:
        "Bug-Catching Contest encounter sentinel must be the final row after 10 weighted rows.",
    },
    {
      name: "extra row after terminator",
      lines: [
        "ContestMons:",
        ...canonicalBugContestEncounterRows,
        "\tdb 1, CATERPIE, 7, 7",
      ],
      message:
        "Bug-Catching Contest encounter table must contain exactly 10 weighted rows and one final sentinel row; found 12 rows.",
    },
  ])(
    "rejects $name instead of emitting a partial encounter table",
    ({ lines, message }) => {
      writeBugContestConfigFixtures({ encounterSourceLines: lines });

      expect(() => exportBugContestConfig()).toThrow(message);
      expect(
        fs.existsSync(
          path.join(mockAssetsRoot, "data", "bug_contest_config.json"),
        ),
      ).toBe(false);
    },
  );

  it("exports Battle Tower banned species from exact ASM ubers check", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "pokemon_constants.asm"),
      [
        "const_def",
        "\tconst BULBASAUR",
        "\tconst MEWTWO",
        "\tconst MEW",
        "\tconst LUGIA",
        "\tconst HO_OH",
        "\tconst CELEBI",
        "DEF NUM_POKEMON EQU const_value - 1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "battle_tower_constants.asm"),
      [
        "DEF BATTLETOWER_PARTY_LENGTH EQU 3",
        "DEF BATTLETOWER_STREAK_LENGTH EQU 7",
        "DEF BATTLETOWER_REWARD_QUANTITY EQU 5",
        "DEF BATTLETOWER_MIN_REWARD EQU HP_UP",
        "DEF BATTLETOWER_MAX_REWARD EQU CALCIUM",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "item_constants.asm"),
      [
        "const_def",
        "\tconst POTION",
        "\tconst HP_UP",
        "\tconst PROTEIN",
        "\tconst IRON",
        "\tconst CARBOS",
        "\tconst LUCKY_PUNCH",
        "\tconst CALCIUM",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "engine",
        "events",
        "battle_tower",
        "battle_tower.asm",
      ),
      [
        "BattleTower_GiveReward:",
        "\tld a, [sBattleTowerReward]",
        "\tld a, POTION",
        "\tret",
        "BattleTowerAction_1C:",
        "\tret",
        "BattleTower_RandomlyChooseReward:",
        ".loop",
        "\tcall Random",
        "\tcp LUCKY_PUNCH",
        "\tjr z, .loop",
        "\tret",
        "BattleTower_SaveOptions:",
        "\tret",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "mobile", "mobile_46.asm"),
      [
        "BattleTower_UbersCheck:",
        ".loop",
        "\tcp MEWTWO",
        "\tjr z, .uber",
        "\tcp MEW",
        "\tjr z, .uber",
        "\tcp LUGIA",
        "\tjr c, .next",
        "\tcp NUM_POKEMON + 1",
        "\tjr nc, .next",
        ".uber",
        "BattleTower_LevelCheck:",
        "\tld c, 10",
        "Strings_L10ToL100:",
        '\tdb " L:10 @@"',
        '\tdb " L:20 @@"',
        '\tdb " L:30 @@"',
        '\tdb " L:40 @@"',
        '\tdb " L:50 @@"',
        '\tdb " L:60 @@"',
        '\tdb " L:70 @@"',
        '\tdb " L:80 @@"',
        '\tdb " L:90 @@"',
        '\tdb " L:100@@"',
        '\tdb "CANCEL@@"',
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "engine",
        "events",
        "battle_tower",
        "rules.asm",
      ),
      [
        "_CheckForBattleTowerRules:",
        "\tld hl, wStringBuffer2",
        "\tld [hl], '3'",
        ".PointerTables:",
        "\tdb 4",
        "\tdw .Functions",
        "\tdw .TextPointers",
        ".TextPointers:",
        "\tdw ExcuseMeYoureNotReadyText",
        "\tdw OnlyThreeMonMayBeEnteredText",
        "\tdw TheMonMustAllBeDifferentKindsText",
        "\tdw TheMonMustNotHoldTheSameItemsText",
        "\tdw YouCantTakeAnEggText",
        "",
      ].join("\n"),
    );

    const rules = exportBattleTowerRules();

    expect(rules).toEqual({
      bannedSpecies: {
        MEWTWO: {},
        MEW: {},
        LUGIA: {},
        HO_OH: {},
        CELEBI: {},
      },
      requiredPartyCount: 3,
      challengeStreakLength: 7,
      rewardCandidates: [
        "HP_UP",
        "PROTEIN",
        "IRON",
        "CARBOS",
        "LUCKY_PUNCH",
        "CALCIUM",
      ],
      excludedRewardItems: ["LUCKY_PUNCH"],
      rewardQuantity: 5,
      rewardFailureSentinel: "POTION",
      rewardItemValues: {
        POTION: 0,
        HP_UP: 1,
        PROTEIN: 2,
        IRON: 3,
        CARBOS: 4,
        LUCKY_PUNCH: 5,
        CALCIUM: 6,
      },
      minimumLevelGroup: 1,
      maximumLevelGroup: 10,
      levelGroupSize: 10,
      partyCountFailureText: "OnlyThreeMonMayBeEnteredText",
      duplicateSpeciesFailureText: "TheMonMustAllBeDifferentKindsText",
      duplicateHeldItemFailureText: "TheMonMustNotHoldTheSameItemsText",
      eggFailureText: "YouCantTakeAnEggText",
      trainers: [
        {
          index: 0,
          trainerClass: "YOUNGSTER",
          name: "JOEY",
          spriteConstant: "SPRITE_YOUNGSTER",
          female: false,
        },
      ],
      monGroups: [
        [
          {
            species: "CHIKORITA",
            item: null,
            moves: ["TACKLE", "NO_MOVE", "NO_MOVE", "NO_MOVE"],
            originalTrainerId: 1234,
            experience: 1000,
            statExp: [0, 0, 0, 0, 0],
            dvs: [8, 8, 8, 8],
            pp: [35, 0, 0, 0],
            happiness: 70,
            pokerus: [0, 0],
            level: 10,
            status: [0, 0],
            stats: [30, 30, 20, 20, 20, 20, 20],
            nickname: "CHIKORITA",
          },
        ],
      ],
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "battle_tower_rules.json"), "utf8"),
      ),
    ).toEqual(rules);
    fs.unlinkSync(
      path.join(mockDisassemblyRoot, "data", "battle_tower", "parties.asm"),
    );
    expect(() => exportBattleTowerRules()).toThrow(
      "Required Battle Tower source is missing: data/battle_tower/parties.asm",
    );
  });

  it("rejects Battle Tower party lengths outside party size range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "pokemon_constants.asm"),
      [
        "const_def",
        "\tconst BULBASAUR",
        "DEF NUM_POKEMON EQU const_value - 1",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "battle_tower_constants.asm"),
      [
        "DEF BATTLETOWER_PARTY_LENGTH EQU 7",
        "DEF BATTLETOWER_STREAK_LENGTH EQU 7",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "mobile", "mobile_46.asm"),
      [
        "BattleTower_UbersCheck:",
        "\tcp BULBASAUR",
        "\tjr c, .next",
        ".uber",
        "BattleTower_LevelCheck:",
        "\tld c, 10",
        "Strings_L10ToL100:",
        '\tdb " L:10 @@"',
        '\tdb "CANCEL@@"',
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "engine",
        "events",
        "battle_tower",
        "rules.asm",
      ),
      [
        "_CheckForBattleTowerRules:",
        "\tld [hl], '7'",
        ".TextPointers:",
        "\tdw ExcuseMeYoureNotReadyText",
        "\tdw OnlyThreeMonMayBeEnteredText",
        "\tdw TheMonMustAllBeDifferentKindsText",
        "\tdw TheMonMustNotHoldTheSameItemsText",
        "\tdw YouCantTakeAnEggText",
        "",
      ].join("\n"),
    );

    expect(() => exportBattleTowerRules()).toThrow(
      "Battle Tower party length 7 is outside party size range.",
    );
  });

  it("rejects Battle Tower level menu entries outside Pokemon range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "pokemon_constants.asm"),
      [
        "const_def",
        "\tconst BULBASAUR",
        "\tconst LUGIA",
        "DEF NUM_POKEMON EQU const_value - 1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "battle_tower_constants.asm"),
      [
        "DEF BATTLETOWER_PARTY_LENGTH EQU 3",
        "DEF BATTLETOWER_STREAK_LENGTH EQU 7",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "mobile", "mobile_46.asm"),
      [
        "BattleTower_UbersCheck:",
        "\tcp LUGIA",
        "\tjr c, .next",
        ".uber",
        "BattleTower_LevelCheck:",
        "\tld c, 10",
        "Strings_L10ToL100:",
        '\tdb " L:110@@"',
        '\tdb "CANCEL@@"',
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "engine",
        "events",
        "battle_tower",
        "rules.asm",
      ),
      [
        "_CheckForBattleTowerRules:",
        "\tld [hl], '3'",
        ".TextPointers:",
        "\tdw ExcuseMeYoureNotReadyText",
        "\tdw OnlyThreeMonMayBeEnteredText",
        "\tdw TheMonMustAllBeDifferentKindsText",
        "\tdw TheMonMustNotHoldTheSameItemsText",
        "\tdw YouCantTakeAnEggText",
        "",
      ].join("\n"),
    );

    expect(() => exportBattleTowerRules()).toThrow(
      "Battle Tower level menu entry 110 is outside Pokemon level range.",
    );
  });

  it("exports Oak rating thresholds from exact ASM table", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "pokedex_ratings.asm"),
      [
        "OakRatings:",
        "\trating   9, SFX_DEX_FANFARE_LESS_THAN_20, OakRating01",
        "\trating  19, SFX_DEX_FANFARE_LESS_THAN_20, OakRating02",
        "\trating 255, SFX_DEX_FANFARE_230_PLUS,     OakRating19",
        "",
      ].join("\n"),
    );

    const ratings = exportOakRatings();

    expect(ratings).toEqual([
      {
        caughtCountLimit: 9,
        fanfare: "SFX_DEX_FANFARE_LESS_THAN_20",
        textLabel: "OakRating01",
      },
      {
        caughtCountLimit: 19,
        fanfare: "SFX_DEX_FANFARE_LESS_THAN_20",
        textLabel: "OakRating02",
      },
      {
        caughtCountLimit: 255,
        fanfare: "SFX_DEX_FANFARE_230_PLUS",
        textLabel: "OakRating19",
      },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "oak_ratings.json"), "utf8"),
      ),
    ).toEqual(ratings);
  });

  it("rejects Oak rating caught-count limits outside byte range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "pokedex_ratings.asm"),
      [
        "OakRatings:",
        "\trating 9, SFX_DEX_FANFARE_LESS_THAN_20, OakRating01",
        "\trating 256, SFX_DEX_FANFARE_230_PLUS, OakRating19",
        "",
      ].join("\n"),
    );

    expect(() => exportOakRatings()).toThrow(
      "Oak rating caught-count limit 256 is outside byte range.",
    );
  });

  it("exports Odd Egg definitions from exact ASM tables", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "odd_egg.asm"),
      [".Odd:", '\tdname "ODD", MON_NAME_LENGTH + 1', ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "odd_eggs.asm"),
      [
        "DEF NUM_ODD_EGGS EQU 1",
        "OddEggProbabilities:",
        "\ttable_width 2",
        "\todd_egg_prob 100",
        "OddEggs:",
        "\ttable_width NICKNAMED_MON_STRUCT_LENGTH",
        "\tdb CLEFFA",
        "\tdb NO_ITEM",
        "\tdb POUND, CHARM, DIZZY_PUNCH, 0",
        "\tdw 00768",
        "\tbigdt 125",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tdn 2, 10, 10, 10",
        "\tdb 35, 20, 10, 0",
        "\tdb 20",
        "\tdb 0, 0, 0",
        "\tdb 5",
        "\tdb 0, 0",
        "\tbigdw 0",
        "\tbigdw 20",
        "\tbigdw 7",
        "\tbigdw 8",
        "\tbigdw 7",
        "\tbigdw 10",
        "\tbigdw 11",
        '\tdname "EGG", MON_NAME_LENGTH',
        "\tassert_table_length NUM_ODD_EGGS",
        "",
      ].join("\n"),
    );

    const definitions = exportOddEggDefinitions();

    expect(definitions).toEqual([
      {
        species: "CLEFFA",
        moves: ["POUND", "CHARM", "DIZZY_PUNCH"],
        originalTrainerId: 768,
        dvs: [2, 10, 10, 10],
        probability: 100,
        level: 5,
        experience: 125,
        hatchCycles: 20,
        nickname: "EGG",
        originalTrainerName: "ODD",
      },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "odd_egg_definitions.json"), "utf8"),
      ),
    ).toEqual(definitions);
  });

  it("rejects Odd Egg DVs outside nibble range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "odd_egg.asm"),
      [".Odd:", '\tdname "ODD", MON_NAME_LENGTH + 1', ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "odd_eggs.asm"),
      [
        "DEF NUM_ODD_EGGS EQU 1",
        "OddEggProbabilities:",
        "\ttable_width 2",
        "\todd_egg_prob 100",
        "OddEggs:",
        "\ttable_width NICKNAMED_MON_STRUCT_LENGTH",
        "\tdb CLEFFA",
        "\tdb NO_ITEM",
        "\tdb POUND, CHARM, DIZZY_PUNCH, 0",
        "\tdw 00768",
        "\tbigdt 125",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tdn 16, 10, 10, 10",
        "\tdb 35, 20, 10, 0",
        "\tdb 20",
        "\tdb 0, 0, 0",
        "\tdb 5",
        "\tdb 0, 0",
        "\tbigdw 0",
        "\tbigdw 20",
        "\tbigdw 7",
        "\tbigdw 8",
        "\tbigdw 7",
        "\tbigdw 10",
        "\tbigdw 11",
        '\tdname "EGG", MON_NAME_LENGTH',
        "\tassert_table_length NUM_ODD_EGGS",
        "",
      ].join("\n"),
    );

    expect(() => exportOddEggDefinitions()).toThrow(
      "Odd Egg DV '16' is outside nibble range.",
    );
  });

  it("rejects Odd Egg probabilities outside percent range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "odd_egg.asm"),
      [".Odd:", '\tdname "ODD", MON_NAME_LENGTH + 1', ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "odd_eggs.asm"),
      [
        "DEF NUM_ODD_EGGS EQU 1",
        "OddEggProbabilities:",
        "\ttable_width 2",
        "\todd_egg_prob 101",
        "OddEggs:",
        "\tassert_table_length NUM_ODD_EGGS",
        "",
      ].join("\n"),
    );

    expect(() => exportOddEggDefinitions()).toThrow(
      "Odd Egg probability 101 is outside percent range.",
    );
  });

  it("rejects Odd Egg move lists exceeding party move limit before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "odd_egg.asm"),
      [".Odd:", '\tdname "ODD", MON_NAME_LENGTH + 1', ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "odd_eggs.asm"),
      [
        "DEF NUM_ODD_EGGS EQU 1",
        "OddEggProbabilities:",
        "\ttable_width 2",
        "\todd_egg_prob 100",
        "OddEggs:",
        "\ttable_width NICKNAMED_MON_STRUCT_LENGTH",
        "\tdb CLEFFA",
        "\tdb NO_ITEM",
        "\tdb POUND, CHARM, DIZZY_PUNCH, LEER, TACKLE",
        "\tdw 00768",
        "\tassert_table_length NUM_ODD_EGGS",
        "",
      ].join("\n"),
    );

    expect(() => exportOddEggDefinitions()).toThrow(
      "Odd Egg move list has 5 moves, exceeding party move limit.",
    );
  });

  it("rejects Odd Egg original trainer ids outside word range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "odd_egg.asm"),
      [".Odd:", '\tdname "ODD", MON_NAME_LENGTH + 1', ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "odd_eggs.asm"),
      [
        "DEF NUM_ODD_EGGS EQU 1",
        "OddEggProbabilities:",
        "\ttable_width 2",
        "\todd_egg_prob 100",
        "OddEggs:",
        "\ttable_width NICKNAMED_MON_STRUCT_LENGTH",
        "\tdb CLEFFA",
        "\tdb NO_ITEM",
        "\tdb POUND, CHARM, DIZZY_PUNCH, 0",
        "\tdw 65536",
        "\tassert_table_length NUM_ODD_EGGS",
        "",
      ].join("\n"),
    );

    expect(() => exportOddEggDefinitions()).toThrow(
      "Odd Egg CLEFFA original trainer id 65536 is outside word range.",
    );
  });

  it("rejects Odd Egg hatch cycles outside byte range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "odd_egg.asm"),
      [".Odd:", '\tdname "ODD", MON_NAME_LENGTH + 1', ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "odd_eggs.asm"),
      [
        "DEF NUM_ODD_EGGS EQU 1",
        "OddEggProbabilities:",
        "\ttable_width 2",
        "\todd_egg_prob 100",
        "OddEggs:",
        "\ttable_width NICKNAMED_MON_STRUCT_LENGTH",
        "\tdb CLEFFA",
        "\tdb NO_ITEM",
        "\tdb POUND, CHARM, DIZZY_PUNCH, 0",
        "\tdw 00768",
        "\tbigdt 125",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tdn 2, 10, 10, 10",
        "\tdb 35, 20, 10, 0",
        "\tdb 256",
        "\tassert_table_length NUM_ODD_EGGS",
        "",
      ].join("\n"),
    );

    expect(() => exportOddEggDefinitions()).toThrow(
      "Odd Egg CLEFFA hatch cycles 256 is outside byte range.",
    );
  });

  it("rejects Odd Egg levels outside Pokemon range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "odd_egg.asm"),
      [".Odd:", '\tdname "ODD", MON_NAME_LENGTH + 1', ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "odd_eggs.asm"),
      [
        "DEF NUM_ODD_EGGS EQU 1",
        "OddEggProbabilities:",
        "\ttable_width 2",
        "\todd_egg_prob 100",
        "OddEggs:",
        "\ttable_width NICKNAMED_MON_STRUCT_LENGTH",
        "\tdb CLEFFA",
        "\tdb NO_ITEM",
        "\tdb POUND, CHARM, DIZZY_PUNCH, 0",
        "\tdw 00768",
        "\tbigdt 125",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tbigdw 0",
        "\tdn 2, 10, 10, 10",
        "\tdb 35, 20, 10, 0",
        "\tdb 20",
        "\tdb 0, 0, 0",
        "\tdb 101",
        "\tassert_table_length NUM_ODD_EGGS",
        "",
      ].join("\n"),
    );

    expect(() => exportOddEggDefinitions()).toThrow(
      "Odd Egg CLEFFA level 101 is outside Pokemon level range.",
    );
  });

  it("exports Magikarp length table from exact ASM dwb rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "magikarp_lengths.asm"),
      [
        "MagikarpLengths:",
        "\tdwb   110, 1",
        "\tdwb   310, 2",
        "\tdwb  65510, 1",
        "",
      ].join("\n"),
    );

    const lengths = exportMagikarpLengths();

    expect(lengths).toEqual([
      { threshold: 110, divisor: 1 },
      { threshold: 310, divisor: 2 },
      { threshold: 65510, divisor: 1 },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "magikarp_lengths.json"), "utf8"),
      ),
    ).toEqual(lengths);
  });

  it("rejects Magikarp length thresholds outside word range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "magikarp_lengths.asm"),
      ["MagikarpLengths:", "\tdwb 65536, 1", ""].join("\n"),
    );

    expect(() => exportMagikarpLengths()).toThrow(
      "Magikarp length threshold 65536 is outside word range.",
    );
  });

  it("rejects Magikarp length byte divisors outside divisor range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "magikarp_lengths.asm"),
      ["MagikarpLengths:", "\tdwb 110, 0", ""].join("\n"),
    );

    expect(() => exportMagikarpLengths()).toThrow(
      "Magikarp length divisor 0 is outside byte divisor range.",
    );
  });

  it("exports happiness changes and service probabilities from exact ASM tables", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "pokemon_data_constants.asm"),
      [
        "\tconst_def 1",
        "\tconst SOME_OTHER_CONSTANT",
        "",
        "; ChangeHappiness arguments (see data/events/happiness_changes.asm)",
        "\tconst_def 1",
        "\tconst HAPPINESS_OLDERCUT1",
        "\tconst HAPPINESS_OLDERCUT2",
        "\tconst HAPPINESS_OLDERCUT3",
        "\tconst HAPPINESS_YOUNGCUT1",
        "\tconst HAPPINESS_YOUNGCUT2",
        "\tconst HAPPINESS_YOUNGCUT3",
        "\tconst HAPPINESS_GROOMING",
        "\tDEF NUM_HAPPINESS_CHANGES EQU const_value - 1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "happiness_changes.asm"),
      [
        "HappinessChanges:",
        "\ttable_width 3, HappinessChanges",
        "\tdb +1, +1, +1",
        "\tdb +3, +3, +1",
        "\tdb +5, +5, +2",
        "\tdb +2, +2, +2",
        "\tdb +4, +4, +2",
        "\tdb +10, +10, +4",
        "\tdb +3, +3, +1",
        "\tassert_table_length NUM_HAPPINESS_CHANGES",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "events",
        "happiness_probabilities.asm",
      ),
      [
        "HappinessData_OlderHaircutBrother:",
        "\tdb 30 percent,     2, HAPPINESS_OLDERCUT1",
        "\tdb 50 percent + 1, 3, HAPPINESS_OLDERCUT2",
        "\tdb -1,             4, HAPPINESS_OLDERCUT3",
        "HappinessData_YoungerHaircutBrother:",
        "\tdb 60 percent + 1, 2, HAPPINESS_YOUNGCUT1",
        "\tdb 30 percent,     3, HAPPINESS_YOUNGCUT2",
        "\tdb -1,             4, HAPPINESS_YOUNGCUT3",
        "HappinessData_DaisysGrooming:",
        "\tdb -1,             2, HAPPINESS_GROOMING",
        "",
      ].join("\n"),
    );

    const data = exportHappinessData();

    expect(data.changes).toEqual({
      "1": { code: "HAPPINESS_OLDERCUT1", low: 1, mid: 1, high: 1 },
      "2": { code: "HAPPINESS_OLDERCUT2", low: 3, mid: 3, high: 1 },
      "3": { code: "HAPPINESS_OLDERCUT3", low: 5, mid: 5, high: 2 },
      "4": { code: "HAPPINESS_YOUNGCUT1", low: 2, mid: 2, high: 2 },
      "5": { code: "HAPPINESS_YOUNGCUT2", low: 4, mid: 4, high: 2 },
      "6": { code: "HAPPINESS_YOUNGCUT3", low: 10, mid: 10, high: 4 },
      "7": { code: "HAPPINESS_GROOMING", low: 3, mid: 3, high: 1 },
    });
    expect(data.services).toEqual({
      OlderHaircutBrother: [
        { rollWeight: 76, scriptValue: 2, changeCode: 1 },
        { rollWeight: 128, scriptValue: 3, changeCode: 2 },
        { rollWeight: 255, scriptValue: 4, changeCode: 3 },
      ],
      YoungerHaircutBrother: [
        { rollWeight: 154, scriptValue: 2, changeCode: 4 },
        { rollWeight: 76, scriptValue: 3, changeCode: 5 },
        { rollWeight: 255, scriptValue: 4, changeCode: 6 },
      ],
      DaisysGrooming: [{ rollWeight: 255, scriptValue: 2, changeCode: 7 }],
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "happiness_data.json"), "utf8"),
      ),
    ).toEqual(data);
  });

  it("rejects duplicate happiness probability tables before pack emission", () => {
    writeMinimalHappinessFixtures([
      "HappinessData_OlderHaircutBrother:",
      "\tdb -1, 2, HAPPINESS_OLDERCUT1",
      "HappinessData_YoungerHaircutBrother:",
      "\tdb -1, 2, HAPPINESS_YOUNGCUT1",
      "HappinessData_DaisysGrooming:",
      "\tdb -1, 2, HAPPINESS_GROOMING",
      "HappinessData_OlderHaircutBrother:",
      "\tdb -1, 3, HAPPINESS_OLDERCUT1",
      "",
    ]);

    expect(() => exportHappinessData()).toThrow(
      "Duplicate happiness probability table 'OlderHaircutBrother'.",
    );
  });

  it("rejects duplicate happiness change constants before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "pokemon_data_constants.asm"),
      [
        "; ChangeHappiness arguments (see data/events/happiness_changes.asm)",
        "\tconst_def 1",
        "\tconst HAPPINESS_OLDERCUT1",
        "\tconst HAPPINESS_OLDERCUT1",
        "\tconst HAPPINESS_GROOMING",
        "\tDEF NUM_HAPPINESS_CHANGES EQU const_value - 1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "happiness_changes.asm"),
      [
        "HappinessChanges:",
        "\tdb +1, +1, +1",
        "\tdb +2, +2, +2",
        "\tdb +3, +3, +1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "events",
        "happiness_probabilities.asm",
      ),
      [
        "HappinessData_OlderHaircutBrother:",
        "\tdb -1, 2, HAPPINESS_OLDERCUT1",
        "HappinessData_YoungerHaircutBrother:",
        "\tdb -1, 2, HAPPINESS_OLDERCUT1",
        "HappinessData_DaisysGrooming:",
        "\tdb -1, 2, HAPPINESS_GROOMING",
        "",
      ].join("\n"),
    );

    expect(() => exportHappinessData()).toThrow(
      "Duplicate happiness change constant 'HAPPINESS_OLDERCUT1'.",
    );
  });

  it("rejects happiness change signed db values outside byte range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "pokemon_data_constants.asm"),
      [
        "; ChangeHappiness arguments (see data/events/happiness_changes.asm)",
        "\tconst_def 1",
        "\tconst HAPPINESS_OLDERCUT1",
        "\tconst HAPPINESS_YOUNGCUT1",
        "\tconst HAPPINESS_GROOMING",
        "\tDEF NUM_HAPPINESS_CHANGES EQU const_value - 1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "happiness_changes.asm"),
      [
        "HappinessChanges:",
        "\tdb +128, +1, +1",
        "\tdb +2, +2, +2",
        "\tdb +3, +3, +1",
        "\tassert_table_length NUM_HAPPINESS_CHANGES",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "events",
        "happiness_probabilities.asm",
      ),
      [
        "HappinessData_OlderHaircutBrother:",
        "\tdb -1, 2, HAPPINESS_OLDERCUT1",
        "HappinessData_YoungerHaircutBrother:",
        "\tdb -1, 2, HAPPINESS_YOUNGCUT1",
        "HappinessData_DaisysGrooming:",
        "\tdb -1, 2, HAPPINESS_GROOMING",
        "",
      ].join("\n"),
    );

    expect(() => exportHappinessData()).toThrow(
      "Signed db number '+128' is outside signed byte range",
    );
  });

  it("rejects happiness probability script values outside byte range before pack emission", () => {
    writeMinimalHappinessFixtures([
      "HappinessData_OlderHaircutBrother:",
      "\tdb -1, 256, HAPPINESS_OLDERCUT1",
      "HappinessData_YoungerHaircutBrother:",
      "\tdb -1, 2, HAPPINESS_YOUNGCUT1",
      "HappinessData_DaisysGrooming:",
      "\tdb -1, 2, HAPPINESS_GROOMING",
      "",
    ]);

    expect(() => exportHappinessData()).toThrow(
      "Happiness probability script value 256 is outside byte range.",
    );
  });

  it("exports encounter slot probability tables from exact ASM mon_prob rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "probabilities.asm"),
      [
        "MACRO mon_prob",
        "\tdb \\1, \\2 * 2",
        "ENDM",
        "",
        "GrassMonProbTable:",
        "\ttable_width 2",
        "\tmon_prob 30,  0 ; 30% chance",
        "\tmon_prob 60,  1 ; 30% chance",
        "\tmon_prob 100, 2 ; 40% chance",
        "\tassert_table_length NUM_GRASSMON",
        "",
        "WaterMonProbTable:",
        "\ttable_width 2",
        "\tmon_prob 60,  0 ; 60% chance",
        "\tmon_prob 90,  1 ; 30% chance",
        "\tmon_prob 100, 2 ; 10% chance",
        "\tassert_table_length NUM_WATERMON",
        "",
      ].join("\n"),
    );

    const tables = exportEncounterSlotTables();

    expect(tables).toEqual({
      tables: {
        grass: [
          { threshold: 30, slot: 0 },
          { threshold: 60, slot: 1 },
          { threshold: 100, slot: 2 },
        ],
        water: [
          { threshold: 60, slot: 0 },
          { threshold: 90, slot: 1 },
          { threshold: 100, slot: 2 },
        ],
      },
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(
          path.join(dataDir, "encounter_slot_tables.json"),
          "utf8",
        ),
      ),
    ).toEqual(tables);
  });

  it("rejects non-increasing encounter slot thresholds before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "probabilities.asm"),
      [
        "GrassMonProbTable:",
        "\tmon_prob 60, 0",
        "\tmon_prob 60, 1",
        "WaterMonProbTable:",
        "\tmon_prob 100, 0",
        "",
      ].join("\n"),
    );

    expect(() => exportEncounterSlotTables()).toThrow(
      "Encounter slot table grass threshold 60 must be greater than 60",
    );
  });

  it("rejects repeated encounter slots before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "probabilities.asm"),
      [
        "GrassMonProbTable:",
        "\tmon_prob 50, 0",
        "\tmon_prob 100, 0",
        "WaterMonProbTable:",
        "\tmon_prob 100, 0",
        "",
      ].join("\n"),
    );

    expect(() => exportEncounterSlotTables()).toThrow(
      "Encounter slot table grass repeats slot 0.",
    );
  });

  it("rejects encounter slots outside byte range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "probabilities.asm"),
      [
        "GrassMonProbTable:",
        "\tmon_prob 100, 256",
        "WaterMonProbTable:",
        "\tmon_prob 100, 0",
        "",
      ].join("\n"),
    );

    expect(() => exportEncounterSlotTables()).toThrow(
      "Encounter slot 256 is outside byte range.",
    );
  });

  it("rejects encounter slot tables that do not end at 100 before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "probabilities.asm"),
      [
        "GrassMonProbTable:",
        "\tmon_prob 50, 0",
        "WaterMonProbTable:",
        "\tmon_prob 100, 0",
        "",
      ].join("\n"),
    );

    expect(() => exportEncounterSlotTables()).toThrow(
      "Encounter slot table grass must end at threshold 100, found 50.",
    );
  });

  it("exports battle stat multiplier tables from exact ASM rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "battle", "stat_multipliers.asm"),
      [
        "; Multiplier ratios for all stats from modifier -6 to +6",
        "\tdb  25, 100 ; -6",
        "\tdb  28, 100",
        "\tdb  33, 100",
        "\tdb  40, 100",
        "\tdb  50, 100",
        "\tdb  66, 100",
        "\tdb   1,   1",
        "\tdb  15,  10",
        "\tdb   2,   1",
        "\tdb  25,  10",
        "\tdb   3,   1",
        "\tdb  35,  10",
        "\tdb   4,   1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "battle",
        "accuracy_multipliers.asm",
      ),
      [
        "AccuracyLevelMultipliers:",
        "\tdb  33, 100 ; -6",
        "\tdb  36, 100",
        "\tdb  43, 100",
        "\tdb  50, 100",
        "\tdb  60, 100",
        "\tdb  75, 100",
        "\tdb   1,   1",
        "\tdb 133, 100",
        "\tdb 166, 100",
        "\tdb   2,   1",
        "\tdb 233, 100",
        "\tdb 133,  50",
        "\tdb   3,   1",
        "",
      ].join("\n"),
    );

    const multipliers = exportBattleStatMultipliers();

    expect(multipliers.stat[0]).toEqual({ numerator: 25, denominator: 100 });
    expect(multipliers.stat[12]).toEqual({ numerator: 4, denominator: 1 });
    expect(multipliers.accuracy[0]).toEqual({
      numerator: 33,
      denominator: 100,
    });
    expect(multipliers.accuracy[8]).toEqual({
      numerator: 166,
      denominator: 100,
    });
    expect(multipliers.accuracy[11]).toEqual({
      numerator: 133,
      denominator: 50,
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(
          path.join(dataDir, "battle_stat_multipliers.json"),
          "utf8",
        ),
      ),
    ).toEqual(multipliers);
  });

  it("rejects battle stat multiplier numerators outside byte range", () => {
    const rows = validBattleStatMultiplierRows();
    rows[0] = "\tdb 256, 1";
    writeBattleStatMultiplierFixtures(rows, validBattleStatMultiplierRows());

    expect(() => exportBattleStatMultipliers()).toThrow(
      "Battle stat multiplier in data/battle/stat_multipliers.asm has numerator 256 outside byte range",
    );
  });

  it("rejects battle stat multiplier denominators outside byte range", () => {
    const rows = validBattleStatMultiplierRows();
    rows[0] = "\tdb 1, 256";
    writeBattleStatMultiplierFixtures(validBattleStatMultiplierRows(), rows);

    expect(() => exportBattleStatMultipliers()).toThrow(
      "Battle stat multiplier in data/battle/accuracy_multipliers.asm has denominator 256 outside byte range",
    );
  });

  it("exports capture wobble probabilities from exact ASM rows", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "battle",
        "wobble_probabilities.asm",
      ),
      [
        "WobbleProbabilities:",
        "; catch rate, chance of wobbling / 255",
        "\tdb   1,  63",
        "\tdb   2,  75",
        "\tdb 254, 253",
        "\tdb 255, 255",
        "",
      ].join("\n"),
    );

    const rows = exportCaptureWobbleProbabilities();

    expect(rows).toEqual([
      { catch_rate: 1, chance: 63 },
      { catch_rate: 2, chance: 75 },
      { catch_rate: 254, chance: 253 },
      { catch_rate: 255, chance: 255 },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(
          path.join(dataDir, "capture_wobble_probabilities.json"),
          "utf8",
        ),
      ),
    ).toEqual(rows);
  });

  it("rejects duplicate capture wobble catch rates before pack emission", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "battle",
        "wobble_probabilities.asm",
      ),
      ["WobbleProbabilities:", "\tdb   1,  63", "\tdb   1,  75", ""].join("\n"),
    );

    expect(() => exportCaptureWobbleProbabilities()).toThrow(
      "Duplicate capture wobble catch rate 1.",
    );
  });

  it("rejects non-increasing capture wobble catch rates before pack emission", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "battle",
        "wobble_probabilities.asm",
      ),
      ["WobbleProbabilities:", "\tdb   2,  75", "\tdb   1,  63", ""].join("\n"),
    );

    expect(() => exportCaptureWobbleProbabilities()).toThrow(
      "Capture wobble catch rate 1 must be greater than 2.",
    );
  });

  it("exports weather type and move-effect modifiers from exact ASM rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "battle", "weather_modifiers.asm"),
      [
        "WeatherTypeModifiers:",
        "\tdb WEATHER_RAIN, WATER, MORE_EFFECTIVE",
        "\tdb WEATHER_RAIN, FIRE,  NOT_VERY_EFFECTIVE",
        "\tdb WEATHER_SUN,  FIRE,  MORE_EFFECTIVE",
        "\tdb WEATHER_SUN,  WATER, NOT_VERY_EFFECTIVE",
        "\tdb -1 ; end",
        "",
        "WeatherMoveModifiers:",
        "\tdb WEATHER_RAIN, EFFECT_SOLARBEAM, NOT_VERY_EFFECTIVE",
        "\tdb -1 ; end",
        "",
      ].join("\n"),
    );

    const modifiers = exportWeatherModifiers();

    expect(modifiers).toEqual({
      type_modifiers: {
        WEATHER_RAIN: {
          WATER: { numerator: 3, denominator: 2 },
          FIRE: { numerator: 1, denominator: 2 },
        },
        WEATHER_SUN: {
          FIRE: { numerator: 3, denominator: 2 },
          WATER: { numerator: 1, denominator: 2 },
        },
      },
      move_effect_modifiers: {
        WEATHER_RAIN: {
          SOLARBEAM: { numerator: 1, denominator: 2 },
        },
      },
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "weather_modifiers.json"), "utf8"),
      ),
    ).toEqual(modifiers);
  });

  it("rejects duplicate weather type modifier rows before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "battle", "weather_modifiers.asm"),
      [
        "WeatherTypeModifiers:",
        "\tdb WEATHER_RAIN, WATER, MORE_EFFECTIVE",
        "\tdb WEATHER_RAIN, WATER, NOT_VERY_EFFECTIVE",
        "\tdb -1",
        "",
        "WeatherMoveModifiers:",
        "\tdb WEATHER_RAIN, EFFECT_SOLARBEAM, NOT_VERY_EFFECTIVE",
        "\tdb -1",
        "",
      ].join("\n"),
    );

    expect(() => exportWeatherModifiers()).toThrow(
      "Duplicate weather type modifier 'WEATHER_RAIN/WATER'.",
    );
  });

  it("rejects duplicate weather move-effect modifier rows before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "battle", "weather_modifiers.asm"),
      [
        "WeatherTypeModifiers:",
        "\tdb WEATHER_RAIN, WATER, MORE_EFFECTIVE",
        "\tdb -1",
        "",
        "WeatherMoveModifiers:",
        "\tdb WEATHER_RAIN, EFFECT_SOLARBEAM, NOT_VERY_EFFECTIVE",
        "\tdb WEATHER_RAIN, EFFECT_SOLARBEAM, MORE_EFFECTIVE",
        "\tdb -1",
        "",
      ].join("\n"),
    );

    expect(() => exportWeatherModifiers()).toThrow(
      "Duplicate weather move-effect modifier 'WEATHER_RAIN/SOLARBEAM'.",
    );
  });

  it("exports type effectiveness as complete type matrix with exact ASM overrides", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "type_constants.asm"),
      [
        "\tconst_def",
        "DEF PHYSICAL EQU const_value",
        "\tconst NORMAL",
        "\tconst FIGHTING",
        "\tconst ROCK",
        "\tconst GHOST",
        "DEF UNUSED_TYPES EQU const_value",
        "\tconst_next 19",
        "\tconst CURSE_TYPE",
        "DEF UNUSED_TYPES_END EQU const_value",
        "DEF SPECIAL EQU const_value",
        "\tconst FIRE",
        "\tconst GRASS",
        "\tconst ELECTRIC",
        "\tconst GROUND",
        "DEF TYPES_END EQU const_value",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "types", "type_matchups.asm"),
      [
        "TypeMatchups:",
        "\tdb NORMAL,       ROCK,         NOT_VERY_EFFECTIVE",
        "\tdb FIRE,         GRASS,        SUPER_EFFECTIVE",
        "\tdb ELECTRIC,     GROUND,       NO_EFFECT",
        "\tdb -2 ; end (with Foresight)",
        "\tdb NORMAL,       GHOST,        NO_EFFECT",
        "\tdb FIGHTING,     GHOST,        NO_EFFECT",
        "\tdb -1 ; end",
        "",
      ].join("\n"),
    );

    const table = exportTypeEffectivenessTable();

    expect(
      Object.values(table.matchups).flatMap((defenders) =>
        Object.values(defenders),
      ),
    ).toHaveLength(64);
    expect(table.matchups.NORMAL.ROCK).toEqual({
      numerator: 1,
      denominator: 2,
    });
    expect(table.matchups.FIRE.GRASS).toEqual({ numerator: 2, denominator: 1 });
    expect(table.matchups.ELECTRIC.GROUND).toEqual({
      numerator: 0,
      denominator: 1,
    });
    expect(table.matchups.NORMAL.NORMAL).toEqual({
      numerator: 1,
      denominator: 1,
    });
    expect(table.foresight_matchups).toEqual({
      NORMAL: { GHOST: { numerator: 0, denominator: 1 } },
      FIGHTING: { GHOST: { numerator: 0, denominator: 1 } },
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "type_effectiveness.json"), "utf8"),
      ),
    ).toEqual(table);
  });

  it("rejects duplicate normal type effectiveness rows before pack emission", () => {
    writeTypeConstantsFixture();
    writeFile(
      path.join(mockDisassemblyRoot, "data", "types", "type_matchups.asm"),
      [
        "TypeMatchups:",
        "\tdb FIRE,         GRASS,        SUPER_EFFECTIVE",
        "\tdb FIRE,         GRASS,        NOT_VERY_EFFECTIVE",
        "\tdb -2 ; end (with Foresight)",
        "\tdb NORMAL,       GHOST,        NO_EFFECT",
        "\tdb -1 ; end",
        "",
      ].join("\n"),
    );

    expect(() => exportTypeEffectivenessTable()).toThrow(
      "Duplicate normal type effectiveness matchup 'FIRE/GRASS'.",
    );
  });

  it("rejects duplicate Foresight type effectiveness rows before pack emission", () => {
    writeTypeConstantsFixture();
    writeFile(
      path.join(mockDisassemblyRoot, "data", "types", "type_matchups.asm"),
      [
        "TypeMatchups:",
        "\tdb FIRE,         GRASS,        SUPER_EFFECTIVE",
        "\tdb -2 ; end (with Foresight)",
        "\tdb NORMAL,       GHOST,        NO_EFFECT",
        "\tdb NORMAL,       GHOST,        NOT_VERY_EFFECTIVE",
        "\tdb -1 ; end",
        "",
      ].join("\n"),
    );

    expect(() => exportTypeEffectivenessTable()).toThrow(
      "Duplicate foresight type effectiveness matchup 'NORMAL/GHOST'.",
    );
  });

  it("rejects type effectiveness rows with unknown attacker types before pack emission", () => {
    writeTypeConstantsFixture();
    writeFile(
      path.join(mockDisassemblyRoot, "data", "types", "type_matchups.asm"),
      [
        "TypeMatchups:",
        "\tdb UNKNOWN_TYPE,  GRASS,        SUPER_EFFECTIVE",
        "\tdb -2 ; end (with Foresight)",
        "\tdb NORMAL,        GHOST,        NO_EFFECT",
        "\tdb -1 ; end",
        "",
      ].join("\n"),
    );

    expect(() => exportTypeEffectivenessTable()).toThrow(
      "Type effectiveness matchup references unknown attacker type 'UNKNOWN_TYPE'.",
    );
  });

  it("rejects type effectiveness rows with unknown defender types before pack emission", () => {
    writeTypeConstantsFixture();
    writeFile(
      path.join(mockDisassemblyRoot, "data", "types", "type_matchups.asm"),
      [
        "TypeMatchups:",
        "\tdb FIRE,          UNKNOWN_TYPE, SUPER_EFFECTIVE",
        "\tdb -2 ; end (with Foresight)",
        "\tdb NORMAL,        GHOST,        NO_EFFECT",
        "\tdb -1 ; end",
        "",
      ].join("\n"),
    );

    expect(() => exportTypeEffectivenessTable()).toThrow(
      "Type effectiveness matchup references unknown defender type 'UNKNOWN_TYPE'.",
    );
  });

  it("exports physical and special type categories from exact ASM constants", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "type_constants.asm"),
      [
        "\tconst_def",
        "",
        "DEF PHYSICAL EQU const_value",
        "\tconst NORMAL",
        "\tconst FIGHTING",
        "\tconst BIRD",
        "DEF UNUSED_TYPES EQU const_value",
        "\tconst_next 19",
        "\tconst CURSE_TYPE",
        "DEF UNUSED_TYPES_END EQU const_value",
        "",
        "DEF SPECIAL EQU const_value",
        "\tconst FIRE",
        "\tconst WATER",
        "\tconst PSYCHIC_TYPE",
        "DEF TYPES_END EQU const_value",
        "",
      ].join("\n"),
    );

    const categories = exportTypeCategories();

    expect(categories).toEqual({
      physical: ["NORMAL", "FIGHTING", "BIRD"],
      special: ["FIRE", "WATER", "PSYCHIC_TYPE"],
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "type_categories.json"), "utf8"),
      ),
    ).toEqual(categories);
  });

  it("rejects duplicate type category constants before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "type_constants.asm"),
      [
        "\tconst_def",
        "DEF PHYSICAL EQU const_value",
        "\tconst NORMAL",
        "\tconst FIRE",
        "DEF UNUSED_TYPES EQU const_value",
        "DEF SPECIAL EQU const_value",
        "\tconst FIRE",
        "DEF TYPES_END EQU const_value",
        "",
      ].join("\n"),
    );

    expect(() => exportTypeCategories()).toThrow(
      "Duplicate type category constant 'FIRE'.",
    );
  });

  it("exports move priority table from exact ASM rows and Vital Throw rule", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "battle_constants.asm"),
      "DEF BASE_PRIORITY EQU 1\n",
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "moves", "effects_priorities.asm"),
      [
        "MoveEffectPriorities:",
        "\tdb EFFECT_PROTECT,      3",
        "\tdb EFFECT_PRIORITY_HIT, 2",
        "\tdb EFFECT_FORCE_SWITCH, 0",
        "\tdb -1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "battle", "core.asm"),
      ["GetMovePriority:", "\tcp VITAL_THROW", "\tld a, 0", "\tret z", ""].join(
        "\n",
      ),
    );

    const priorities = exportMovePriorityTable({
      TACKLE: { effect: "NORMAL_HIT" },
      QUICK_ATTACK: { effect: "PRIORITY_HIT" },
      ROAR: { effect: "FORCE_SWITCH" },
    } as any);

    expect(priorities).toEqual({
      base_priority: 1,
      effect_priorities: {
        FORCE_SWITCH: 0,
        NORMAL_HIT: 1,
        PRIORITY_HIT: 2,
      },
      move_priorities: [{ move: "VITAL_THROW", priority: 0 }],
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(
      JSON.parse(
        fs.readFileSync(path.join(dataDir, "move_priorities.json"), "utf8"),
      ),
    ).toEqual(priorities);
  });

  it("rejects duplicate move effect priority rows before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "battle_constants.asm"),
      "DEF BASE_PRIORITY EQU 1\n",
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "moves", "effects_priorities.asm"),
      [
        "MoveEffectPriorities:",
        "\tdb EFFECT_PRIORITY_HIT, 2",
        "\tdb EFFECT_PRIORITY_HIT, 3",
        "\tdb -1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "battle", "core.asm"),
      ["GetMovePriority:", "\tcp VITAL_THROW", "\tld a, 0", "\tret z", ""].join(
        "\n",
      ),
    );

    expect(() => exportMovePriorityTable({} as any)).toThrow(
      "Duplicate move effect priority 'PRIORITY_HIT'.",
    );
  });

  it("rejects BASE_PRIORITY values outside byte range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "battle_constants.asm"),
      "DEF BASE_PRIORITY EQU 256\n",
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "moves", "effects_priorities.asm"),
      [
        "MoveEffectPriorities:",
        "\tdb EFFECT_PRIORITY_HIT, 2",
        "\tdb -1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "battle", "core.asm"),
      ["GetMovePriority:", "\tcp VITAL_THROW", "\tld a, 0", "\tret z", ""].join(
        "\n",
      ),
    );

    expect(() => exportMovePriorityTable({} as any)).toThrow(
      "BASE_PRIORITY 256 is outside byte range.",
    );
  });

  it("rejects move effect priority values outside byte range before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "battle_constants.asm"),
      "DEF BASE_PRIORITY EQU 1\n",
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "moves", "effects_priorities.asm"),
      [
        "MoveEffectPriorities:",
        "\tdb EFFECT_PRIORITY_HIT, 256",
        "\tdb -1",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "battle", "core.asm"),
      ["GetMovePriority:", "\tcp VITAL_THROW", "\tld a, 0", "\tret z", ""].join(
        "\n",
      ),
    );

    expect(() => exportMovePriorityTable({} as any)).toThrow(
      "Move effect priority 'EFFECT_PRIORITY_HIT' value 256 is outside byte range.",
    );
  });

  it("requires exact lowercase frontpic animation opcodes without case coercion", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tFRAME 1, 07", "\tendanim", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Unknown frontpic animation opcode 'FRAME'",
    );
  });

  it("rejects malformed frontpic animation rows instead of skipping them", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tframe 1", "\tendanim", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Malformed frontpic animation frame row",
    );
  });

  it("rejects frontpic animation numeric operands outside byte range", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tframe 256, 05", "\tendanim", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Frontpic animation numeric operand '256' is outside byte range",
    );
  });

  it("rejects negative frontpic animation numeric operands", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tframe -1, 05", "\tendanim", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Frontpic animation numeric operand '-1' is outside byte range",
    );
  });

  it("rejects frontpic dorepeat commands without setrepeat", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tframe 0, 05", "\tdorepeat 0", "\tendanim", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Frontpic animation dorepeat requires setrepeat.",
    );
  });

  it("rejects frontpic dorepeat targets that are not earlier commands", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tsetrepeat 2", "\tdorepeat 2", "\tendanim", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Frontpic animation dorepeat target 2 does not reference an earlier command.",
    );
  });

  it("rejects frontpic setrepeat commands without dorepeat", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tsetrepeat 2", "\tframe 0, 05", "\tendanim", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Frontpic animation setrepeat is missing dorepeat.",
    );
  });

  it("rejects zero frontpic setrepeat counts", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tsetrepeat 0", "\tframe 0, 05", "\tdorepeat 0", "\tendanim", ""].join(
        "\n",
      ),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Frontpic animation setrepeat count must be nonzero.",
    );
  });

  it("rejects frontpic animation programs without endanim", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tframe 0, 05", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Frontpic animation program is missing endanim.",
    );
  });

  it("rejects frontpic animation commands after endanim", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tframe 0, 05", "\tendanim", "\tframe 1, 05", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Frontpic animation program has commands after endanim.",
    );
  });

  it("rejects unknown Pokedex entry file stems instead of normalizing them", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "pokemon",
        "dex_entries",
        "mr-mime.asm",
      ),
      [
        '\tdb "BARRIER@"',
        "\tdw 403, 1200 ; height, weight",
        '\tdb "It mimics@"',
        "",
      ].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Unknown or case-changed runtime species file stem 'mr-mime'",
    );
  });

  it("rejects Pokedex entries without text pages before pack emission", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "pokemon",
        "dex_entries",
        "chikorita.asm",
      ),
      ['\tdb "LEAF@"', "\tdw 209, 140 ; height, weight", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow("Pokedex entry for CHIKORITA");
    expect(() => exportRuntimeAssets()).toThrow(
      "must declare nonempty text pages",
    );
  });

  it("rejects Pokedex entries with empty pages before page breaks", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "pokemon",
        "dex_entries",
        "chikorita.asm",
      ),
      [
        '\tdb "LEAF@"',
        "\tdw 209, 140 ; height, weight",
        '\tpage "Second page.@"',
        "",
      ].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "has an empty text page before page break",
    );
  });

  it("rejects malformed Pokedex text rows before pack emission", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "pokemon",
        "dex_entries",
        "chikorita.asm",
      ),
      [
        '\tdb "LEAF@"',
        "\tdw 209, 140 ; height, weight",
        '\tbadtext "Leafy.@"',
        "",
      ].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Malformed Pokedex entry text row for CHIKORITA",
    );
  });

  it("rejects Pokedex entries with empty classifications before pack emission", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "pokemon",
        "dex_entries",
        "chikorita.asm",
      ),
      ['\tdb "@"', "\tdw 209, 140 ; height, weight", '\tdb "Leafy.@"', ""].join(
        "\n",
      ),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "must declare a nonempty classification",
    );
  });

  it("rejects Pokedex entries with height digits outside display range", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "pokemon",
        "dex_entries",
        "chikorita.asm",
      ),
      [
        '\tdb "LEAF@"',
        "\tdw 100000, 140 ; height, weight",
        '\tdb "Leafy.@"',
        "",
      ].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "has size digits outside supported display range",
    );
  });

  it("rejects Pokedex entries with weight digits outside display range", () => {
    writeFile(
      path.join(
        mockDisassemblyRoot,
        "data",
        "pokemon",
        "dex_entries",
        "chikorita.asm",
      ),
      [
        '\tdb "LEAF@"',
        "\tdw 209, 100000 ; height, weight",
        '\tdb "Leafy.@"',
        "",
      ].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "has size digits outside supported display range",
    );
  });

  it("rejects duplicate Pokemon species constants before runtime species maps are built", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "pokemon_constants.asm"),
      [
        "\tconst_def 1",
        "\tconst CHIKORITA",
        "\tconst CHIKORITA",
        "\tconst EGG",
        "",
      ].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Duplicate Pokemon species constant 'CHIKORITA'.",
    );
  });

  it("rejects unknown frontpic animation directories instead of normalizing them", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "mr-mime", "anim.asm"),
      ["\tframe 0, 05", "\tendanim", ""].join("\n"),
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Unknown or case-changed runtime species file stem 'mr-mime'",
    );
  });

  it("requires permanent phone numbers to resolve to exact declared contacts or trainer labels", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "permanent_numbers.asm"),
      "\tdb PHONECONTACT_ELM\n\tdb -1\n",
    );

    expect(() => exportPermanentPhoneNumbers()).toThrow(
      "Permanent phone number 'PHONECONTACT_ELM' does not match a declared phone contact id or trainer label",
    );
  });

  it("rejects duplicate phone contact constants before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "phone_constants.asm"),
      [
        "\tconst PHONE_00",
        "\tconst PHONE_MOM",
        "\tconst PHONE_MOM",
        "\tDEF NUM_PHONE_CONTACTS EQU const_value",
        "; SpecialPhoneCallList indexes (see data/phone/special_calls.asm)",
        "\tconst_def",
        "\tconst SPECIALCALL_NONE",
        "\tDEF NUM_SPECIALCALLS EQU const_value - 1",
        "",
      ].join("\n"),
    );

    expect(() => exportPhoneContacts()).toThrow(
      "Phone contact constant 'PHONE_MOM' is declared more than once.",
    );
  });

  it("rejects duplicate non-trainer phone labels before pack emission", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "non_trainer_names.asm"),
      ['.mom: db "MOM:@"', '.mom: db "MOTHER:@"', ""].join("\n"),
    );

    expect(() => exportPhoneContacts()).toThrow(
      "Duplicate non-trainer phone label 'PHONECONTACT_MOM'.",
    );
  });

  it("rejects lowercase hex phone sentinels instead of coercing them", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "permanent_numbers.asm"),
      "\tdb $ff\n",
    );

    expect(() => exportPermanentPhoneNumbers()).toThrow(
      "Permanent phone number '$ff' does not match a declared phone contact id or trainer label",
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
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "phone", "phone_contacts.asm"),
      [
        "\tphone TRAINER_NONE, 0, 0, 0, 0, 0, 0",
        "\tphone TRAINER_NONE, PHONECONTACT_MOM, PLAYERS_HOUSE_1F, ANYTIME, MomPhoneCalleeScript, 0, 0",
        "\tphone YOUNGSTER, JOEY1, ROUTE_30, ANYTIME, JoeyPhoneCalleeScript, ANYTIME, JoeyPhoneCallerScript",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "trainer_constants.asm"),
      ["trainerclass TRAINER_NONE", "trainerclass YOUNGSTER", ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "trainers", "class_names.asm"),
      "",
    );

    expect(() => exportRuntimeAssets()).toThrow(
      "Trainer class id count 1 does not match class name count 0.",
    );
  });

  it("exports trainer class display names in exact constant-table order", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "trainer_constants.asm"),
      [
        "trainerclass TRAINER_NONE",
        "trainerclass COOLTRAINERM",
        "trainerclass POKEMON_PROF",
        "trainerclass CAL",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "trainers", "class_names.asm"),
      [
        '\tli "COOLTRAINER"',
        '\tli "#MON PROF."',
        '\tli "<PKMN> TRAINER"',
        "",
      ].join("\n"),
    );

    const names = exportTrainerClassNames();

    expect(names).toEqual({
      COOLTRAINERM: "COOLTRAINER",
      POKEMON_PROF: "POKéMON PROF.",
      CAL: "PKMN TRAINER",
    });
    expect(Object.keys(names)).toEqual(["COOLTRAINERM", "POKEMON_PROF", "CAL"]);
    expect(
      JSON.parse(
        fs.readFileSync(
          path.join(mockAssetsRoot, "data", "trainer_class_names.json"),
          "utf8",
        ),
      ),
    ).toEqual(names);
  });

  it("requires TRAINER_NONE to anchor the trainer class constant table", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "trainer_constants.asm"),
      ["trainerclass YOUNGSTER", ""].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "trainers", "class_names.asm"),
      ['\tli "YOUNGSTER"', ""].join("\n"),
    );

    expect(() => exportTrainerClassNames()).toThrow(
      "Trainer class constant table must begin with exact TRAINER_NONE, found 'YOUNGSTER'.",
    );
  });

  it("rejects duplicate trainer class ids before phone contact export", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "trainer_constants.asm"),
      [
        "trainerclass TRAINER_NONE",
        "trainerclass YOUNGSTER",
        "trainerclass YOUNGSTER",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "trainers", "class_names.asm"),
      ['\tli "YOUNGSTER@"', '\tli "YOUNGSTER@"', ""].join("\n"),
    );

    expect(() => exportPhoneContacts()).toThrow(
      "Duplicate trainer class id 'YOUNGSTER'.",
    );
  });

  it("rejects mismatched trainer class id and name counts before phone contact export", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "trainer_constants.asm"),
      [
        "trainerclass TRAINER_NONE",
        "trainerclass YOUNGSTER",
        "trainerclass BUG_CATCHER",
        "",
      ].join("\n"),
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "trainers", "class_names.asm"),
      ['\tli "YOUNGSTER@"', ""].join("\n"),
    );

    expect(() => exportPhoneContacts()).toThrow(
      "Trainer class id count 2 does not match class name count 1.",
    );
  });
});

describe("timeTokenToMask", () => {
  it("requires exact phone time-mask tokens without case coercion or unknown-token fallback", () => {
    expect(timeTokenToMask("ANYTIME")).toBe(7);
    expect(timeTokenToMask("MORN|NITE")).toBe(5);
    expect(timeTokenToMask("0")).toBe(0);
    expect(timeTokenToMask("8")).toBe(8);

    expect(() => timeTokenToMask("anytime")).toThrow(
      "Unknown phone time mask token 'anytime'",
    );
    expect(() => timeTokenToMask("MORN|late")).toThrow(
      "Unknown phone time mask token 'late' in 'MORN|late'",
    );
  });
});
