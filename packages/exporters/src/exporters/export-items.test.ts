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

const completeAttributeTableWithFirstRow = (name: string, row: string): string => {
  const rows = [[`; ${name}`, row].join("\n")];
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

const completeItemEffects = (): string =>
  [
    "ItemEffects:",
    "\tdw ReviveEffect        ; REVIVE",
    "\tdw ReviveEffect        ; MAX_REVIVE",
    "\tdw RevivalHerbEffect   ; REVIVAL_HERB",
    "\tdw SacredAshEffect     ; SACRED_ASH",
    "\tdw RestorePPEffect     ; MAX_ELIXER",
    "\tdw RestorePPEffect     ; ETHER",
    "\tdw RestorePPEffect     ; MAX_ETHER",
    "\tdw RestorePPEffect     ; ELIXER",
    "\tdw EscapeRopeEffect    ; ESCAPE_ROPE",
    "\tdw PokeDollEffect      ; POKE_DOLL",
    "\tdw GuardSpecEffect     ; GUARD_SPEC",
    "\tdw DireHitEffect       ; DIRE_HIT",
    "\tdw BitterBerryEffect   ; BITTER_BERRY",
    "",
    "RevivalHerbEffect:",
    "\tcall RevivePokemon",
    "",
    "ReviveEffect:",
    "\tcall RevivePokemon",
    "",
    "RevivePokemon:",
    "\tld a, [wCurItem]",
    "\tcp REVIVE",
    "\tjr z, .revive_half_hp",
    "\tcall ReviveFullHP",
    "\tjr .finish_revive",
    ".revive_half_hp",
    "\tcall ReviveHalfHP",
    ".finish_revive",
    "\tret",
    "",
    "ReviveHalfHP:",
    "\tcall LoadHPFromBuffer1",
    "\tsrl d",
    "\trr e",
    "\tjr ContinueRevive",
    "",
    "ReviveFullHP:",
    "\tcall LoadHPFromBuffer1",
    "",
    "SacredAshEffect:",
    "\tfarcall _SacredAsh",
    "",
    "RestorePPEffect:",
    "\tld a, [wTempRestorePPItem]",
    "\tcp MAX_ELIXER",
    "\tjp z, Elixer_RestorePPofAllMoves",
    "\tcp ELIXER",
    "\tjp z, Elixer_RestorePPofAllMoves",
    "\tjp Not_PP_Up",
    "",
    "Elixer_RestorePPofAllMoves:",
    "\tjp BattleRestorePP",
    "",
    "EscapeRopeEffect:",
    "\txor a",
    "\tld [wItemEffectSucceeded], a",
    "\tfarcall EscapeRopeFunction",
    "\tret",
    "",
    "PokeDollEffect:",
    "\tld a, [wBattleMode]",
    "\tdec a ; WILD_BATTLE?",
    "\tjr nz, .not_wild",
    "\tinc a ; TRUE",
    "\tld [wForcedSwitch], a",
    "\tld a, [wBattleResult]",
    "\tand BATTLERESULT_BITMASK",
    "\tor DRAW",
    "\tld [wBattleResult], a",
    "\tjp UseItemText",
    ".not_wild",
    "\txor a",
    "\tld [wItemEffectSucceeded], a",
    "\tret",
    "",
    "GuardSpecEffect:",
    "\tld hl, wPlayerSubStatus4",
    "\tbit SUBSTATUS_MIST, [hl]",
    "\tjp nz, WontHaveAnyEffect_NotUsedMessage",
    "\tset SUBSTATUS_MIST, [hl]",
    "\tjp UseItemText",
    "",
    "DireHitEffect:",
    "\tld hl, wPlayerSubStatus4",
    "\tbit SUBSTATUS_FOCUS_ENERGY, [hl]",
    "\tjp nz, WontHaveAnyEffect_NotUsedMessage",
    "\tset SUBSTATUS_FOCUS_ENERGY, [hl]",
    "\tjp UseItemText",
    "",
    "BitterBerryEffect:",
    "\tld hl, wPlayerSubStatus3",
    "\tbit SUBSTATUS_CONFUSED, [hl]",
    "\tld a, 1",
    "\tjr z, .done",
    "\tres SUBSTATUS_CONFUSED, [hl]",
    "\txor a",
    ".done",
    "\tjp StatusHealer_Jumptable",
    "",
    "VitaminEffect:",
    "\tld a, [hl]",
    "\tcp 100",
    "\tjr nc, NoEffectMessage",
    "\tadd hl, bc",
    "\tadd 10",
    "\tld [hl], a",
    "",
    "StatExpItemPointerOffsets:",
    "\tdb HP_UP,    MON_HP_EXP - MON_STAT_EXP",
    "\tdb PROTEIN, MON_ATK_EXP - MON_STAT_EXP",
    "\tdb IRON,    MON_DEF_EXP - MON_STAT_EXP",
    "\tdb CARBOS,  MON_SPD_EXP - MON_STAT_EXP",
    "\tdb CALCIUM, MON_SPC_EXP - MON_STAT_EXP",
    "",
    "RareCandyEffect:",
    "\tld a, MON_LEVEL",
    "\tcall GetPartyParamLocation",
    "\tld a, [hl]",
    "\tcp MAX_LEVEL",
    "\tjp nc, NoEffectMessage",
    "\tinc a",
    "\tld [hl], a",
    "",
    "SuperRepelEffect:",
    "\tld b, 200",
    "\tjr UseRepel",
    "",
    "MaxRepelEffect:",
    "\tld b, 250",
    "\tjr UseRepel",
    "",
    "RepelEffect:",
    "\tld b, 100",
    "",
    "UseRepel:",
    "\tld a, [wRepelEffect]",
    "",
  ].join("\n");

const itemEffectsTableScaffold = (): string =>
  [
    "ItemEffects:",
    "\tdw ReviveEffect        ; REVIVE",
    "\tdw ReviveEffect        ; MAX_REVIVE",
    "\tdw RevivalHerbEffect   ; REVIVAL_HERB",
    "\tdw SacredAshEffect     ; SACRED_ASH",
    "\tdw RestorePPEffect     ; MAX_ELIXER",
    "\tdw RestorePPEffect     ; ETHER",
    "\tdw RestorePPEffect     ; MAX_ETHER",
    "\tdw RestorePPEffect     ; ELIXER",
    "\tdw EscapeRopeEffect    ; ESCAPE_ROPE",
    "\tdw PokeDollEffect      ; POKE_DOLL",
    "\tdw GuardSpecEffect     ; GUARD_SPEC",
    "\tdw DireHitEffect       ; DIRE_HIT",
    "\tdw BitterBerryEffect   ; BITTER_BERRY",
    "",
    "RevivalHerbEffect:",
    "\tcall RevivePokemon",
    "",
    "ReviveEffect:",
    "\tcall RevivePokemon",
    "",
    "RevivePokemon:",
    "\tld a, [wCurItem]",
    "\tcp REVIVE",
    "\tjr z, .revive_half_hp",
    "\tcall ReviveFullHP",
    "",
    "ReviveHalfHP:",
    "\tcall LoadHPFromBuffer1",
    "\tsrl d",
    "\trr e",
    "",
    "ReviveFullHP:",
    "\tcall LoadHPFromBuffer1",
    "",
    "SacredAshEffect:",
    "\tfarcall _SacredAsh",
    "",
    "RestorePPEffect:",
    "\tld a, [wTempRestorePPItem]",
    "\tcp MAX_ELIXER",
    "\tjp z, Elixer_RestorePPofAllMoves",
    "\tcp ELIXER",
    "\tjp z, Elixer_RestorePPofAllMoves",
    "\tjp Not_PP_Up",
    "",
    "Elixer_RestorePPofAllMoves:",
    "\tjp BattleRestorePP",
    "",
    "EscapeRopeEffect:",
    "\tfarcall EscapeRopeFunction",
    "\tret",
    "",
    "PokeDollEffect:",
    "\tld a, [wBattleMode]",
    "\tdec a ; WILD_BATTLE?",
    "\tor DRAW",
    "\tld [wBattleResult], a",
    "",
    "GuardSpecEffect:",
    "\tbit SUBSTATUS_MIST, [hl]",
    "\tset SUBSTATUS_MIST, [hl]",
    "",
    "DireHitEffect:",
    "\tset SUBSTATUS_FOCUS_ENERGY, [hl]",
    "",
    "BitterBerryEffect:",
    "\tbit SUBSTATUS_CONFUSED, [hl]",
    "\tres SUBSTATUS_CONFUSED, [hl]",
    "",
  ].join("\n");

const completeSacredAshScript = (): string =>
  ["_SacredAsh:", "\tret", "", "SacredAshScript:", "\tspecial HealParty", "\tend", ""].join("\n");

const completeOverworldScript = (): string =>
  [
    "EscapeRopeFunction:",
    "\tcall FieldMoveJumptableReset",
    "\tld a, $1",
    "\tjr EscapeRopeOrDig",
    "",
    "DigFunction:",
    "\tcall FieldMoveJumptableReset",
    "\tld a, $2",
    "",
    "EscapeRopeOrDig:",
    "\tld [wEscapeRopeOrDigType], a",
    ".loop",
    "\tld hl, .DigTable",
    "\tcall FieldMoveJumptable",
    "\tjr nc, .loop",
    "\tand JUMPTABLE_INDEX_MASK",
    "\tld [wFieldMoveSucceeded], a",
    "\tret",
    ".DigTable:",
    "\tdw .CheckCanDig",
    "\tdw .DoDig",
    "\tdw .FailDig",
    ".DoDig:",
    "\tld hl, wDigWarpNumber",
    "\tld de, wNextWarp",
    "\tld bc, 3",
    "\tcall CopyBytes",
    "\tld hl, .UsedEscapeRopeScript",
    "\tcall QueueScript",
    "\tld a, JUMPTABLE_EXIT | $1",
    "\tret",
    ".UsedEscapeRopeScript:",
    "\trefreshmap",
    "\twritetext .UseEscapeRopeText",
    "\tsjump .UsedDigOrEscapeRopeScript",
    ".UsedDigOrEscapeRopeScript:",
    "\twaitbutton",
    "\tclosetext",
    "\tplaysound SFX_WARP_TO",
    "\tapplymovement PLAYER, .DigOut",
    "\tfarscall Script_AbortBugContest",
    "\tspecial WarpToSpawnPoint",
    "\tloadvar VAR_MOVEMENT, PLAYER_NORMAL",
    "\tnewloadmap MAPSETUP_DOOR",
    "\tend",
    "",
  ].join("\n");

const completeHealHpTable = (): string =>
  [
    "HealingHPAmounts:",
    "\tdbw FRESH_WATER,   50",
    "\tdbw SODA_POP,      60",
    "\tdbw LEMONADE,      80",
    "\tdbw HYPER_POTION, 200",
    "\tdbw SUPER_POTION,  50",
    "\tdbw POTION,        20",
    "\tdbw MAX_POTION,   MAX_STAT_VALUE",
    "\tdbw FULL_RESTORE, MAX_STAT_VALUE",
    "\tdbw MOOMOO_MILK,  100",
    "\tdbw BERRY,         10",
    "\tdbw GOLD_BERRY,    30",
    "\tdbw ENERGYPOWDER,  50",
    "\tdbw ENERGY_ROOT,  200",
    "\tdbw RAGECANDYBAR,  20",
    "\tdbw BERRY_JUICE,   20",
    "\tdbw -1, 0 ; end",
    "",
  ].join("\n");

const completeHealStatusTable = (): string =>
  [
    "StatusHealingActions:",
    "\tdb ANTIDOTE,     PARTYMENUTEXT_HEAL_PSN, 1 << PSN",
    "\tdb BURN_HEAL,    PARTYMENUTEXT_HEAL_BRN, 1 << BRN",
    "\tdb ICE_HEAL,     PARTYMENUTEXT_HEAL_FRZ, 1 << FRZ",
    "\tdb AWAKENING,    PARTYMENUTEXT_HEAL_SLP, SLP_MASK",
    "\tdb PARLYZ_HEAL,  PARTYMENUTEXT_HEAL_PAR, 1 << PAR",
    "\tdb FULL_HEAL,    PARTYMENUTEXT_HEAL_ALL, %11111111",
    "\tdb FULL_RESTORE, PARTYMENUTEXT_HEAL_ALL, %11111111",
    "\tdb HEAL_POWDER,  PARTYMENUTEXT_HEAL_ALL, %11111111",
    "\tdb PSNCUREBERRY, PARTYMENUTEXT_HEAL_PSN, 1 << PSN",
    "\tdb PRZCUREBERRY, PARTYMENUTEXT_HEAL_PAR, 1 << PAR",
    "\tdb BURNT_BERRY,  PARTYMENUTEXT_HEAL_FRZ, 1 << FRZ",
    "\tdb ICE_BERRY,    PARTYMENUTEXT_HEAL_BRN, 1 << BRN",
    "\tdb MINT_BERRY,   PARTYMENUTEXT_HEAL_SLP, SLP_MASK",
    "\tdb MIRACLEBERRY, PARTYMENUTEXT_HEAL_ALL, %11111111",
    "\tdb -1, 0, 0 ; end",
    "",
  ].join("\n");

const completeXStatsTable = (): string =>
  [
    "XItemStats:",
    "\tdb X_ATTACK,  ATTACK",
    "\tdb X_DEFEND,  DEFENSE",
    "\tdb X_SPEED,   SPEED",
    "\tdb X_SPECIAL, SP_ATTACK",
    "",
  ].join("\n");

describe("exportItems", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-items-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");

    writeFile(path.join(mockDisassemblyRoot, "constants", "item_constants.asm"), "");
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "battle_constants.asm"),
      [
        "DEF MAX_STAT_VALUE EQU 999",
        "DEF SLP_MASK EQU %111",
        "\tconst_def 3",
        "\tconst PSN",
        "\tconst BRN",
        "\tconst FRZ",
        "\tconst PAR",
        "",
      ].join("\n")
    );
    writeFile(path.join(mockDisassemblyRoot, "constants", "pokemon_data_constants.asm"), "DEF PP_UP_ONE EQU %01000000\n");
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "descriptions.asm"), completeDescriptionTable());
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "heal_hp.asm"), completeHealHpTable());
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "heal_status.asm"), completeHealStatusTable());
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "x_stats.asm"), completeXStatsTable());
    writeFile(path.join(mockDisassemblyRoot, "engine", "events", "overworld.asm"), completeOverworldScript());
    writeFile(path.join(mockDisassemblyRoot, "engine", "events", "sacred_ash.asm"), completeSacredAshScript());
    writeFile(path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"), completeItemEffects());
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

  it("exports vitamin stat metadata as definitive modpack fields", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("PROTEIN"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "PROTEIN",
      effect: "VITAMIN",
      vitamin_stat: "ATTACK",
      vitamin_stat_exp: 2560,
      vitamin_max_stat_exp: 25600,
      party_revive_hp_percent: null,
      rare_candy_level_gain: null,
      battle_stat_boost_stat: null,
      battle_stat_boost_stages: null,
      pp_up_stages: null,
    });
  });

  it("requires vitamin metadata to come from authored item effect ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("PROTEIN"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      [
        itemEffectsTableScaffold(),
        "VitaminEffect:",
        "\tld a, [hl]",
        "\tcp 100",
        "\tadd hl, bc",
        "\tadd 10",
        "",
        "StatExpItemPointerOffsets:",
        "\tdb HP_UP, MON_HP_EXP - MON_STAT_EXP",
        "",
        "RareCandyEffect:",
        "\tld a, MON_LEVEL",
        "\tcall GetPartyParamLocation",
        "\tld a, [hl]",
        "\tinc a",
        "\tld [hl], a",
        "",
        "SuperRepelEffect:",
        "\tld b, 200",
        "",
        "MaxRepelEffect:",
        "\tld b, 250",
        "",
        "RepelEffect:",
        "\tld b, 100",
        "",
      ].join("\n")
    );

    expect(() => exportItems()).toThrow("missing authored vitamin_stat for vitamin item PROTEIN");
  });

  it("exports Rare Candy level gain as definitive modpack data", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("RARE_CANDY"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "RARE_CANDY",
      effect: "RARE_CANDY",
      rare_candy_level_gain: 1,
      party_revive_hp_percent: null,
      vitamin_stat: null,
      vitamin_stat_exp: null,
      vitamin_max_stat_exp: null,
      battle_stat_boost_stat: null,
      battle_stat_boost_stages: null,
    });
  });

  it("requires Rare Candy level gain to come from authored item effect ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("RARE_CANDY"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      [
        itemEffectsTableScaffold(),
        "VitaminEffect:",
        "\tld a, [hl]",
        "\tcp 100",
        "\tadd hl, bc",
        "\tadd 10",
        "",
        "StatExpItemPointerOffsets:",
        "\tdb HP_UP, MON_HP_EXP - MON_STAT_EXP",
        "\tdb PROTEIN, MON_ATK_EXP - MON_STAT_EXP",
        "\tdb IRON, MON_DEF_EXP - MON_STAT_EXP",
        "\tdb CARBOS, MON_SPD_EXP - MON_STAT_EXP",
        "\tdb CALCIUM, MON_SPC_EXP - MON_STAT_EXP",
        "",
        "RareCandyEffect:",
        "\tld a, MON_LEVEL",
        "\tcall GetPartyParamLocation",
        "\tld a, [hl]",
        "\tld [hl], a",
        "",
        "SuperRepelEffect:",
        "\tld b, 200",
        "",
        "MaxRepelEffect:",
        "\tld b, 250",
        "",
        "RepelEffect:",
        "\tld b, 100",
        "",
      ].join("\n")
    );

    expect(() => exportItems()).toThrow("missing authored Rare Candy level increment in RareCandyEffect");
  });

  it("exports PP Up stages from authored PP constants", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("PP_UP"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "PP_UP",
      effect: "PP_UP",
      pp_up_stages: 1,
      pp_restore_scope: null,
      pp_restore_points: null,
    });
  });

  it("requires PP Up stages to come from authored PP constants", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("PP_UP"));
    writeFile(path.join(mockDisassemblyRoot, "constants", "pokemon_data_constants.asm"), "");

    expect(() => exportItems()).toThrow("missing ASM constant PP_UP_ONE for PP Up stages");
  });

  it("exports single-move PP restore scope from authored RestorePPEffect ASM", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"),
      completeAttributeTableWithFirstRow("ETHER", "\titem_attribute 1200, HELD_NONE, 10, CANT_SELECT, ITEM, ITEMMENU_PARTY, ITEMMENU_PARTY")
    );

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "ETHER",
      effect: "RESTORE_PP",
      pp_restore_scope: "MOVE",
      pp_restore_points: 10,
      status_heals: [],
    });
  });

  it("exports all-move PP restore scope from authored RestorePPEffect ASM", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"),
      completeAttributeTableWithFirstRow("ELIXER", "\titem_attribute 3000, HELD_NONE, 10, CANT_SELECT, ITEM, ITEMMENU_PARTY, ITEMMENU_PARTY")
    );

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "ELIXER",
      effect: "RESTORE_PP",
      pp_restore_scope: "POKEMON",
      pp_restore_points: 10,
      status_heals: [],
    });
  });

  it("requires PP restore scope to come from authored RestorePPEffect table rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"),
      completeAttributeTableWithFirstRow("ETHER", "\titem_attribute 1200, HELD_NONE, 10, CANT_SELECT, ITEM, ITEMMENU_PARTY, ITEMMENU_PARTY")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      completeItemEffects().replace("\tdw RestorePPEffect     ; ETHER\n", "")
    );

    expect(() => exportItems()).toThrow("missing authored pp_restore_scope for PP item ETHER");
  });

  it("exports HP restore amounts as definitive modpack data", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("ENERGY_ROOT"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "ENERGY_ROOT",
      effect: "RESTORE_HP",
      parameter: 200,
      revive_hp_percent: null,
      status_heals: [],
      pp_restore_scope: null,
    });
  });

  it("requires HP restore amounts to come from authored heal HP ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("ENERGY_ROOT"));
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "heal_hp.asm"),
      ["HealingHPAmounts:", "\tdbw POTION, 20", "\tdbw -1, 0", ""].join("\n")
    );

    expect(() => exportItems()).toThrow("missing authored HP restore parameter for item ENERGY_ROOT");
  });

  it("exports full restore HP as definitive full-heal data", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("FULL_RESTORE"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "FULL_RESTORE",
      effect: "FULL_RESTORE",
      parameter: -1,
      status_heals: ["POISON", "BURN", "FREEZE", "SLEEP", "PARALYSIS"],
      revive_hp_percent: null,
    });
  });

  it("exports single status-heal metadata from authored status healing ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("ANTIDOTE"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "ANTIDOTE",
      effect: "STATUS_HEAL",
      status_heals: ["POISON"],
      revive_hp_percent: null,
      pp_restore_scope: null,
    });
  });

  it("exports all-status heal metadata from authored status healing ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("HEAL_POWDER"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "HEAL_POWDER",
      effect: "STATUS_HEAL",
      status_heals: ["POISON", "BURN", "FREEZE", "SLEEP", "PARALYSIS"],
      revive_hp_percent: null,
      pp_restore_scope: null,
    });
  });

  it("requires status-heal metadata to come from authored status healing ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("ANTIDOTE"));
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "heal_status.asm"),
      ["StatusHealingActions:", "\tdb BURN_HEAL, PARTYMENUTEXT_HEAL_BRN, 1 << BRN", "\tdb -1, 0, 0", ""].join("\n")
    );

    expect(() => exportItems()).toThrow("missing authored status_heals for status item ANTIDOTE");
  });

  it("exports revive HP percentages from authored revive ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("REVIVE"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "REVIVE",
      effect: "REVIVE",
      revive_hp_percent: 50,
      party_revive_hp_percent: null,
    });
  });

  it("exports full revive items from authored revive ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("MAX_REVIVE"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "MAX_REVIVE",
      effect: "REVIVE",
      revive_hp_percent: 100,
      party_revive_hp_percent: null,
    });
  });

  it("requires revive HP percentages to come from authored revive ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("REVIVE"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      completeItemEffects().replace("\tcp REVIVE\n\tjr z, .revive_half_hp", "\tcp REVIVE")
    );

    expect(() => exportItems()).toThrow("missing authored half-revive item branch in RevivePokemon");
  });

  it("exports battle stat boost metadata as definitive modpack fields", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("X_SPECIAL"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "X_SPECIAL",
      effect: "X_ITEM",
      battle_stat_boost_stat: "SPECIAL_ATTACK",
      battle_stat_boost_stages: 1,
      party_revive_hp_percent: null,
      rare_candy_level_gain: null,
      vitamin_stat: null,
    });
  });

  it("requires X item stat metadata to come from authored x_stats ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("X_SPECIAL"));
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "x_stats.asm"), ["XItemStats:", "\tdb X_ATTACK, ATTACK", ""].join("\n"));

    expect(() => exportItems()).toThrow("missing authored battle_stat_boost_stat for battle boost item X_SPECIAL");
  });

  it("exports Sacred Ash party revive percent as definitive modpack data", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("SACRED_ASH"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "SACRED_ASH",
      effect: "SACRED_ASH",
      party_revive_hp_percent: 100,
      revive_hp_percent: null,
      rare_candy_level_gain: null,
    });
  });

  it("requires Sacred Ash party revive percent to come from authored HealParty script", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("SACRED_ASH"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "sacred_ash.asm"),
      completeSacredAshScript().replace("\tspecial HealParty", "\tplaysound SFX_WARP_TO")
    );

    expect(() => exportItems()).toThrow("missing authored Sacred Ash HealParty script");
  });

  it("exports battle escape item metadata as definitive modpack data", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("POKE_DOLL"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "POKE_DOLL",
      effect: "POKE_DOLL",
      battle_escape_mode: "WILD_BATTLE",
      battle_focus_energy: null,
      battle_stat_drop_guard: null,
      battle_stat_drop_guard_turns: null,
      confusion_heal: null,
    });
  });

  it("requires battle escape mode to come from authored Poke Doll ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("POKE_DOLL"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      completeItemEffects().replace("\tor DRAW\n\tld [wBattleResult], a", "\tld [wBattleResult], a")
    );

    expect(() => exportItems()).toThrow("missing authored Poke Doll draw battle result");
  });

  it("exports Dire Hit focus-energy metadata as definitive modpack data", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("DIRE_HIT"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "DIRE_HIT",
      effect: "DIRE_HIT",
      battle_escape_mode: null,
      battle_focus_energy: true,
      battle_stat_drop_guard: null,
      battle_stat_drop_guard_turns: null,
      confusion_heal: null,
    });
  });

  it("requires Dire Hit focus-energy metadata to come from authored item effect ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("DIRE_HIT"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      completeItemEffects().replace("\tset SUBSTATUS_FOCUS_ENERGY, [hl]", "\tres SUBSTATUS_FOCUS_ENERGY, [hl]")
    );

    expect(() => exportItems()).toThrow("missing authored Dire Hit focus energy substatus");
  });

  it("exports Guard Spec stat-drop guard metadata from authored item effect ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("GUARD_SPEC"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "GUARD_SPEC",
      effect: "GUARD_SPEC",
      battle_escape_mode: null,
      battle_focus_energy: null,
      battle_stat_drop_guard: true,
      battle_stat_drop_guard_turns: 5,
      confusion_heal: null,
    });
  });

  it("requires Guard Spec stat-drop guard metadata to come from authored item effect ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("GUARD_SPEC"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      completeItemEffects().replace("\tset SUBSTATUS_MIST, [hl]", "\tres SUBSTATUS_MIST, [hl]")
    );

    expect(() => exportItems()).toThrow("missing authored Guard Spec mist substatus");
  });

  it("exports Bitter Berry confusion-heal metadata as definitive modpack data", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("BITTER_BERRY"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "BITTER_BERRY",
      effect: "BITTER_BERRY",
      battle_escape_mode: null,
      battle_focus_energy: null,
      battle_stat_drop_guard: null,
      battle_stat_drop_guard_turns: null,
      confusion_heal: true,
    });
  });

  it("requires Bitter Berry confusion-heal metadata to come from authored item effect ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("BITTER_BERRY"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      completeItemEffects().replace("\tres SUBSTATUS_CONFUSED, [hl]", "\tset SUBSTATUS_CONFUSED, [hl]")
    );

    expect(() => exportItems()).toThrow("missing authored Bitter Berry confusion heal");
  });

  it("exports Repel step counts as definitive modpack data", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("SUPER_REPEL"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "SUPER_REPEL",
      effect: "SUPER_REPEL",
      repel_steps: 200,
      battle_escape_mode: null,
      battle_focus_energy: null,
      battle_stat_drop_guard: null,
      battle_stat_drop_guard_turns: null,
      confusion_heal: null,
    });
  });

  it("requires repel step counts to come from authored item effect ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("SUPER_REPEL"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      [
        itemEffectsTableScaffold(),
        "VitaminEffect:",
        "\tld a, [hl]",
        "\tcp 100",
        "\tadd hl, bc",
        "\tadd 10",
        "",
        "StatExpItemPointerOffsets:",
        "\tdb HP_UP, MON_HP_EXP - MON_STAT_EXP",
        "\tdb PROTEIN, MON_ATK_EXP - MON_STAT_EXP",
        "\tdb IRON, MON_DEF_EXP - MON_STAT_EXP",
        "\tdb CARBOS, MON_SPD_EXP - MON_STAT_EXP",
        "\tdb CALCIUM, MON_SPC_EXP - MON_STAT_EXP",
        "",
        "RareCandyEffect:",
        "\tld a, MON_LEVEL",
        "\tcall GetPartyParamLocation",
        "\tld a, [hl]",
        "\tinc a",
        "\tld [hl], a",
        "",
        "SuperRepelEffect:",
        "\tjr UseRepel",
        "",
        "MaxRepelEffect:",
        "\tld b, 250",
        "",
        "RepelEffect:",
        "\tld b, 100",
        "",
      ].join("\n")
    );

    expect(() => exportItems()).toThrow("missing authored repel step count in SuperRepelEffect");
  });

  it("exports Escape Rope mode as definitive modpack data", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("ESCAPE_ROPE"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "ESCAPE_ROPE",
      effect: "ESCAPE_ROPE",
      escape_rope_mode: "DIG_WARP",
      repel_steps: null,
      battle_escape_mode: null,
      battle_focus_energy: null,
      battle_stat_drop_guard: null,
      battle_stat_drop_guard_turns: null,
      confusion_heal: null,
    });
  });

  it("requires Escape Rope mode to come from authored overworld warp ASM", () => {
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), completeAttributeTable("ESCAPE_ROPE"));
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "overworld.asm"),
      completeOverworldScript().replace("\tspecial WarpToSpawnPoint", "\tspecial FadeOutPalettes")
    );

    expect(() => exportItems()).toThrow("missing authored Escape Rope/Dig spawn warp");
  });

  it("exports Blue Card effect as definitive modpack data", () => {
    const rows = [
      ["; BLUE_CARD", "\titem_attribute 0, HELD_NONE, 0, CANT_SELECT, KEY_ITEM, ITEMMENU_CLOSE, ITEMMENU_NOUSE"].join("\n"),
    ];
    for (let index = 1; index < 256; index += 1) {
      rows.push(unusedAttributeRow(`ITEM_${index.toString(16).toUpperCase().padStart(2, "0")}`));
    }
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), `${rows.join("\n")}\n`);

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "BLUE_CARD",
      effect: "BLUE_CARD",
      pocket: "KEY_ITEM",
      consumable: false,
    });
  });

  it("exports TM/HM taught moves from item constants as definitive modpack data", () => {
    const rows = [
      ["; TM01", "\titem_attribute 3000, HELD_NONE, 0, CANT_SELECT, TM_HM, ITEMMENU_PARTY, ITEMMENU_NOUSE"].join("\n"),
      ["; HM01", "\titem_attribute 0, HELD_NONE, 0, CANT_SELECT, TM_HM, ITEMMENU_PARTY, ITEMMENU_NOUSE"].join("\n"),
    ];
    for (let index = 2; index < 256; index += 1) {
      rows.push(unusedAttributeRow(`ITEM_${index.toString(16).toUpperCase().padStart(2, "0")}`));
    }
    writeFile(path.join(mockDisassemblyRoot, "data", "items", "attributes.asm"), `${rows.join("\n")}\n`);
    const tmhmConstants = ["\tadd_tm DYNAMICPUNCH"];
    for (let index = 2; index <= 50; index += 1) {
      tmhmConstants.push("\tadd_tm HEADBUTT");
    }
    tmhmConstants.push("\tadd_hm CUT", "");
    writeFile(path.join(mockDisassemblyRoot, "constants", "item_constants.asm"), tmhmConstants.join("\n"));

    const items = exportItems();

    expect(items[0]).toMatchObject({
      script_name: "TM_DYNAMICPUNCH",
      effect: "NONE",
      pocket: "TM_HM",
      consumable: true,
      tmhm_index: 1,
      tmhm_move: "DYNAMICPUNCH",
    });
    expect(items[1]).toMatchObject({
      script_name: "HM_CUT",
      effect: "NONE",
      pocket: "TM_HM",
      consumable: false,
      tmhm_index: 51,
      tmhm_move: "CUT",
    });
  });
});
