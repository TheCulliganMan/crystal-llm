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
  exportMagikarpLengths,
  exportMovePriorityTable,
  exportOakRatings,
  exportOddEggDefinitions,
  exportPermanentPhoneNumbers,
  exportRoamingPokemon,
  exportRuntimeAssets,
  exportShuckieGift,
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

  it("exports roaming Pokemon definitions from InitRoamMons using exact runtime map metadata", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "overworld", "wildmons.asm"),
      [
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
        "CheckEncounterRoamMon:",
        "",
      ].join("\n")
    );

    const roamers = exportRoamingPokemon({
      ROUTE_42: { groupId: 2, mapId: 5 },
      ROUTE_37: { groupId: 10, mapId: 4 },
    });

    expect(roamers).toEqual([
      { species: "RAIKOU", level: 40, mapGroup: 2, mapNumber: 5 },
      { species: "ENTEI", level: 40, mapGroup: 10, mapNumber: 4 },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "roaming_pokemon.json"), "utf8"))).toEqual(roamers);
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
      ].join("\n")
    );

    const prizes = exportBuenaPrizes();

    expect(prizes).toEqual([
      { itemId: "ULTRA_BALL", cost: 2 },
      { itemId: "RARE_CANDY", cost: 3 },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "buena_prizes.json"), "utf8"))).toEqual(prizes);
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
        '.HealingItems:  db BUENA_ITEM,   12, POTION, ANTIDOTE, PARLYZ_HEAL',
        '.RadioStations: db BUENA_STRING, 13, "#MON Talk@", "#MON Music@", "Lucky Channel@"',
        "",
      ].join("\n")
    );

    const categories = exportBuenaPasswordCategories();

    expect(categories).toEqual([
      {
        id: "HealingItems",
        categoryType: "BUENA_ITEM",
        points: 12,
        options: ["POTION", "ANTIDOTE", "PARLYZ_HEAL"],
      },
      {
        id: "RadioStations",
        categoryType: "BUENA_STRING",
        points: 13,
        options: ["#MON Talk", "#MON Music", "Lucky Channel"],
      },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "buena_password_categories.json"), "utf8"))).toEqual(categories);
  });

  it("exports Kurt apricorn recipes from exact ASM apricorn/ball rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "apricorn_balls.asm"),
      ["ApricornBalls:", "\tdb RED_APRICORN, LEVEL_BALL", "\tdb BLU_APRICORN, LURE_BALL", "\tdb -1", ""].join("\n")
    );

    const recipes = exportKurtApricornRecipes();

    expect(recipes).toEqual([
      { apricorn: "RED_APRICORN", ball: "LEVEL_BALL" },
      { apricorn: "BLU_APRICORN", ball: "LURE_BALL" },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "kurt_apricorn_recipes.json"), "utf8"))).toEqual(recipes);
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
      ].join("\n")
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
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "shuckie_gift.json"), "utf8"))).toEqual(gift);
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
      ].join("\n")
    );

    const moveSets = exportDratiniMoveSets();

    expect(moveSets).toEqual([
      { mode: 0, moves: ["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"] },
      { mode: 1, moves: ["WRAP", "LEER", "THUNDER_WAVE", "TWISTER"] },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "dratini_move_sets.json"), "utf8"))).toEqual(moveSets);
  });

  it("exports Bug-Catching Contest config from exact ASM constants and flag table", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "script_constants.asm"),
      [
        "DEF BUG_CONTEST_BALLS EQU 20",
        "DEF BUG_CONTEST_MINUTES EQU 20",
        "DEF BUG_CONTEST_SECONDS EQU 0",
        "DEF NUM_BUG_CONTESTANTS EQU 3",
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "bug_contest", "contest_2.asm"),
      [
        "SelectRandomBugContestContestants:",
        ".loop1",
        "\tld c, NUM_BUG_CONTESTANTS",
        "\tld c, 2",
        ".loop2",
        "CheckBugContestContestantFlag:",
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "bug_contest_flags.asm"),
      [
        "BugCatchingContestantEventFlagTable:",
        "\ttable_width 2",
        "\tdw EVENT_BUG_CATCHING_CONTESTANT_1A",
        "\tdw EVENT_BUG_CATCHING_CONTESTANT_2A",
        "\tdw EVENT_BUG_CATCHING_CONTESTANT_3A",
        "\tassert_table_length NUM_BUG_CONTESTANTS",
        "",
      ].join("\n")
    );

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
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "bug_contest_config.json"), "utf8"))).toEqual(config);
  });

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
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "battle_tower_constants.asm"),
      ["DEF BATTLETOWER_PARTY_LENGTH EQU 3", "DEF BATTLETOWER_STREAK_LENGTH EQU 7", ""].join("\n")
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
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "battle_tower", "rules.asm"),
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
      ].join("\n")
    );

    const rules = exportBattleTowerRules();

    expect(rules).toEqual({
      bannedSpecies: ["MEWTWO", "MEW", "LUGIA", "HO_OH", "CELEBI"],
      requiredPartyCount: 3,
      challengeStreakLength: 7,
      minimumLevelGroup: 1,
      maximumLevelGroup: 10,
      levelGroupSize: 10,
      partyCountFailureText: "OnlyThreeMonMayBeEnteredText",
      duplicateSpeciesFailureText: "TheMonMustAllBeDifferentKindsText",
      duplicateHeldItemFailureText: "TheMonMustNotHoldTheSameItemsText",
      eggFailureText: "YouCantTakeAnEggText",
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "battle_tower_rules.json"), "utf8"))).toEqual(rules);
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
      ].join("\n")
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
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "oak_ratings.json"), "utf8"))).toEqual(ratings);
  });

  it("exports Odd Egg definitions from exact ASM tables", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "odd_egg.asm"),
      ['.Odd:', '\tdname "ODD", MON_NAME_LENGTH + 1', ""].join("\n")
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
      ].join("\n")
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
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "odd_egg_definitions.json"), "utf8"))).toEqual(definitions);
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
      ].join("\n")
    );

    const lengths = exportMagikarpLengths();

    expect(lengths).toEqual([
      { threshold: 110, divisor: 1 },
      { threshold: 310, divisor: 2 },
      { threshold: 65510, divisor: 1 },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "magikarp_lengths.json"), "utf8"))).toEqual(lengths);
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
      ].join("\n")
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
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "events", "happiness_probabilities.asm"),
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
      ].join("\n")
    );

    const data = exportHappinessData();

    expect(data.changes).toEqual([
      { code: "HAPPINESS_OLDERCUT1", changeCode: 1, low: 1, mid: 1, high: 1 },
      { code: "HAPPINESS_OLDERCUT2", changeCode: 2, low: 3, mid: 3, high: 1 },
      { code: "HAPPINESS_OLDERCUT3", changeCode: 3, low: 5, mid: 5, high: 2 },
      { code: "HAPPINESS_YOUNGCUT1", changeCode: 4, low: 2, mid: 2, high: 2 },
      { code: "HAPPINESS_YOUNGCUT2", changeCode: 5, low: 4, mid: 4, high: 2 },
      { code: "HAPPINESS_YOUNGCUT3", changeCode: 6, low: 10, mid: 10, high: 4 },
      { code: "HAPPINESS_GROOMING", changeCode: 7, low: 3, mid: 3, high: 1 },
    ]);
    expect(data.services).toEqual([
      {
        routine: "OlderHaircutBrother",
        outcomes: [
          { rollWeight: 76, scriptValue: 2, changeCode: 1 },
          { rollWeight: 128, scriptValue: 3, changeCode: 2 },
          { rollWeight: 255, scriptValue: 4, changeCode: 3 },
        ],
      },
      {
        routine: "YoungerHaircutBrother",
        outcomes: [
          { rollWeight: 154, scriptValue: 2, changeCode: 4 },
          { rollWeight: 76, scriptValue: 3, changeCode: 5 },
          { rollWeight: 255, scriptValue: 4, changeCode: 6 },
        ],
      },
      {
        routine: "DaisysGrooming",
        outcomes: [{ rollWeight: 255, scriptValue: 2, changeCode: 7 }],
      },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "happiness_data.json"), "utf8"))).toEqual(data);
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
      ].join("\n")
    );

    const tables = exportEncounterSlotTables();

    expect(tables).toEqual({
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
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "encounter_slot_tables.json"), "utf8"))).toEqual(tables);
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
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "battle", "accuracy_multipliers.asm"),
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
      ].join("\n")
    );

    const multipliers = exportBattleStatMultipliers();

    expect(multipliers.stat[0]).toEqual({ numerator: 25, denominator: 100 });
    expect(multipliers.stat[12]).toEqual({ numerator: 4, denominator: 1 });
    expect(multipliers.accuracy[0]).toEqual({ numerator: 33, denominator: 100 });
    expect(multipliers.accuracy[8]).toEqual({ numerator: 166, denominator: 100 });
    expect(multipliers.accuracy[11]).toEqual({ numerator: 133, denominator: 50 });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "battle_stat_multipliers.json"), "utf8"))).toEqual(
      multipliers
    );
  });

  it("exports capture wobble probabilities from exact ASM rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "battle", "wobble_probabilities.asm"),
      [
        "WobbleProbabilities:",
        "; catch rate, chance of wobbling / 255",
        "\tdb   1,  63",
        "\tdb   2,  75",
        "\tdb 254, 253",
        "\tdb 255, 255",
        "",
      ].join("\n")
    );

    const rows = exportCaptureWobbleProbabilities();

    expect(rows).toEqual([
      { catch_rate: 1, chance: 63 },
      { catch_rate: 2, chance: 75 },
      { catch_rate: 254, chance: 253 },
      { catch_rate: 255, chance: 255 },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "capture_wobble_probabilities.json"), "utf8"))).toEqual(
      rows
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
      ].join("\n")
    );

    const modifiers = exportWeatherModifiers();

    expect(modifiers).toEqual({
      type_modifiers: [
        { weather: "WEATHER_RAIN", move_type: "WATER", multiplier: { numerator: 3, denominator: 2 } },
        { weather: "WEATHER_RAIN", move_type: "FIRE", multiplier: { numerator: 1, denominator: 2 } },
        { weather: "WEATHER_SUN", move_type: "FIRE", multiplier: { numerator: 3, denominator: 2 } },
        { weather: "WEATHER_SUN", move_type: "WATER", multiplier: { numerator: 1, denominator: 2 } },
      ],
      move_effect_modifiers: [
        {
          weather: "WEATHER_RAIN",
          move_effect: "EFFECT_SOLARBEAM",
          multiplier: { numerator: 1, denominator: 2 },
        },
      ],
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "weather_modifiers.json"), "utf8"))).toEqual(modifiers);
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
      ].join("\n")
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
      ].join("\n")
    );

    const table = exportTypeEffectivenessTable();

    expect(table.matchups).toHaveLength(64);
    expect(table.matchups).toContainEqual({
      attacker: "NORMAL",
      defender: "ROCK",
      multiplier: { numerator: 1, denominator: 2 },
    });
    expect(table.matchups).toContainEqual({
      attacker: "FIRE",
      defender: "GRASS",
      multiplier: { numerator: 2, denominator: 1 },
    });
    expect(table.matchups).toContainEqual({
      attacker: "ELECTRIC",
      defender: "GROUND",
      multiplier: { numerator: 0, denominator: 1 },
    });
    expect(table.matchups).toContainEqual({
      attacker: "NORMAL",
      defender: "NORMAL",
      multiplier: { numerator: 1, denominator: 1 },
    });
    expect(table.foresight_matchups).toEqual([
      { attacker: "NORMAL", defender: "GHOST", multiplier: { numerator: 0, denominator: 1 } },
      { attacker: "FIGHTING", defender: "GHOST", multiplier: { numerator: 0, denominator: 1 } },
    ]);
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "type_effectiveness.json"), "utf8"))).toEqual(table);
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
      ].join("\n")
    );

    const categories = exportTypeCategories();

    expect(categories).toEqual({
      physical: ["NORMAL", "FIGHTING", "BIRD"],
      special: ["FIRE", "WATER", "PSYCHIC_TYPE"],
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "type_categories.json"), "utf8"))).toEqual(categories);
  });

  it("exports move priority table from exact ASM rows and Vital Throw rule", () => {
    writeFile(path.join(mockDisassemblyRoot, "constants", "battle_constants.asm"), "DEF BASE_PRIORITY EQU 1\n");
    writeFile(
      path.join(mockDisassemblyRoot, "data", "moves", "effects_priorities.asm"),
      [
        "MoveEffectPriorities:",
        "\tdb EFFECT_PROTECT,      3",
        "\tdb EFFECT_PRIORITY_HIT, 2",
        "\tdb EFFECT_FORCE_SWITCH, 0",
        "\tdb -1",
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "battle", "core.asm"),
      ["GetMovePriority:", "\tcp VITAL_THROW", "\tld a, 0", "\tret z", ""].join("\n")
    );

    const priorities = exportMovePriorityTable({
      TACKLE: { effect: "NORMAL_HIT" },
      QUICK_ATTACK: { effect: "PRIORITY_HIT" },
      ROAR: { effect: "FORCE_SWITCH" },
    } as any);

    expect(priorities).toEqual({
      base_priority: 1,
      effect_priorities: [
        { move_effect: "FORCE_SWITCH", priority: 0 },
        { move_effect: "NORMAL_HIT", priority: 1 },
        { move_effect: "PRIORITY_HIT", priority: 2 },
      ],
      move_priorities: [{ move: "VITAL_THROW", priority: 0 }],
    });
    const dataDir = path.join(mockAssetsRoot, "data");
    expect(JSON.parse(fs.readFileSync(path.join(dataDir, "move_priorities.json"), "utf8"))).toEqual(priorities);
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
