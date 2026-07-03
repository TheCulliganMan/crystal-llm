import fs from "fs";
import os from "os";
import path from "path";
import { exportFieldMoves } from "./export-field-moves";

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

describe("exportFieldMoves", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-field-moves-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports exact badge, collision, and replacement tables without reusing local labels", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "ram_constants.asm"),
      [
        "; wJohtoBadges::",
        "\tconst_def",
        "\tconst ZEPHYRBADGE",
        "\tconst HIVEBADGE",
        "\tconst PLAINBADGE",
        "\tconst FOGBADGE",
        "\tconst MINERALBADGE",
        "\tconst STORMBADGE",
        "\tconst GLACIERBADGE",
        "\tconst RISINGBADGE",
        "DEF NUM_JOHTO_BADGES EQU const_value",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "move_constants.asm"),
      [
        "\tconst FLY",
        "\tconst DIG",
        "\tconst TELEPORT",
        "\tconst HEADBUTT",
        "\tconst ROCK_SMASH",
        "\tconst SWEET_SCENT",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "collision_constants.asm"),
      [
        "DEF COLL_CUT_TREE EQU $12",
        "DEF COLL_CUT_TREE_1A EQU $1a",
        "DEF COLL_TALL_GRASS EQU $18",
        "DEF COLL_LONG_GRASS EQU $14",
        "DEF COLL_LONG_GRASS_1C EQU $1c",
        "DEF COLL_HEADBUTT_TREE EQU $15",
        "DEF COLL_HEADBUTT_TREE_1D EQU $1d",
        "DEF COLL_WHIRLPOOL EQU $24",
        "DEF COLL_WHIRLPOOL_2C EQU $2c",
        "DEF COLL_WATERFALL_RIGHT EQU $30",
        "DEF COLL_WATERFALL_LEFT EQU $31",
        "DEF COLL_WATERFALL_UP EQU $32",
        "DEF COLL_WATERFALL EQU $33",
        "DEF COLL_CURRENT_DOWN EQU $3b",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "collision", "field_move_blocks.asm"),
      [
        "CutTreeBlockPointers:",
        "\tdbw TILESET_JOHTO, .johto",
        "\tdb -1",
        ".johto:",
        "\tdb $5b, $3c, 0",
        "\tdb -1",
        "WhirlpoolBlockPointers:",
        "\tdbw TILESET_JOHTO, .johto",
        "\tdb -1",
        ".johto:",
        "\tdb $07, $36, 0",
        "\tdb -1",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      [
        "ItemEffects:",
        "\tdw TownMapEffect       ; TOWN_MAP",
        "\tdw RepelEffect         ; REPEL",
        "\tdw SuperRepelEffect    ; SUPER_REPEL",
        "\tdw MaxRepelEffect      ; MAX_REPEL",
        "\tdw BicycleEffect       ; BICYCLE",
        "\tdw CoinCaseEffect      ; COIN_CASE",
        "\tdw ItemfinderEffect    ; ITEMFINDER",
        "\tdw BlueCardEffect      ; BLUE_CARD",
        "\tdw SquirtbottleEffect  ; SQUIRTBOTTLE",
        "\tdw EscapeRopeEffect    ; ESCAPE_ROPE",
        "TownMapEffect:",
        "\tfarcall PokegearMap",
        "\tret",
        "RepelEffect:",
        "\tld b, 100",
        "\tret",
        "SuperRepelEffect:",
        "\tld b, 200",
        "\tret",
        "MaxRepelEffect:",
        "\tld b, 250",
        "\tret",
        "BicycleEffect:",
        "\tfarcall BikeFunction",
        "\tret",
        "CoinCaseEffect:",
        "\tld hl, .CoinCaseCountText",
        "\tjp MenuTextboxWaitButton",
        ".CoinCaseCountText:",
        "\ttext_far _CoinCaseCountText",
        "\ttext_end",
        "ItemfinderEffect:",
        "\tfarcall ItemFinder",
        "\tret",
        "BlueCardEffect:",
        "\tld hl, .BlueCardBalanceText",
        "\tjp MenuTextboxWaitButton",
        ".BlueCardBalanceText:",
        "\ttext_far _BlueCardBalanceText",
        "\ttext_end",
        "SquirtbottleEffect:",
        "\tfarcall _Squirtbottle",
        "\tret",
        "EscapeRopeEffect:",
        "\tfarcall EscapeRopeFunction",
        "\tret",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "overworld.asm"),
      [
        "EscapeRopeFunction:",
        "\tld a, $1",
        "\tjr EscapeRopeOrDig",
        "EscapeRopeOrDig:",
        "\tld [wEscapeRopeOrDigType], a",
        "\tcall QueueScript",
        "\tret",
        ".UsedDigOrEscapeRopeScript:",
        "\tspecial WarpToSpawnPoint",
        "\tret",
      ].join("\n")
    );

    const catalog = exportFieldMoves();

    expect(catalog.cut.badge).toEqual({ region: "johto", index: 1 });
    expect(catalog.flash.engine_flag).toBe("STATUSFLAGS_FLASH");
    expect(catalog.fly).toEqual({ move_id: "FLY", badge: { region: "johto", index: 5 } });
    expect(catalog.dig).toEqual({ move_id: "DIG", target_collisions: [] });
    expect(catalog.teleport).toEqual({ move_id: "TELEPORT", target_collisions: [] });
    expect(catalog.headbutt).toEqual({ move_id: "HEADBUTT", target_collisions: [0x15, 0x1d] });
    expect(catalog.rock_smash).toEqual({ move_id: "ROCK_SMASH", target_collisions: [] });
    expect(catalog.sweet_scent).toEqual({ move_id: "SWEET_SCENT", target_collisions: [] });
    expect(catalog.escape_rope).toEqual({ item_id: "ESCAPE_ROPE", escape_rope_mode: "DIG_WARP" });
    expect(catalog.repel).toEqual({});
    expect(catalog.bicycle).toEqual({ item_id: "BICYCLE" });
    expect(catalog.itemfinder).toEqual({ item_id: "ITEMFINDER" });
    expect(catalog.squirtbottle).toEqual({ item_id: "SQUIRTBOTTLE" });
    expect(catalog.coin_case).toEqual({ item_id: "COIN_CASE" });
    expect(catalog.blue_card).toEqual({ item_id: "BLUE_CARD" });
    expect(catalog.town_map).toEqual({ item_id: "TOWN_MAP" });
    expect(catalog.surf.blocked_collisions).toEqual([0x24, 0x2c, 0x33, 0x30, 0x31, 0x32]);
    expect(catalog.waterfall.target_collisions).toEqual([0x33, 0x30, 0x31, 0x32, 0x3b]);
    expect(catalog.cut.replacements).toEqual({
      johto: {
        "91": { replacement_block_id: 0x3c, variant: "tree" },
      },
    });
    expect(catalog.whirlpool.replacements).toEqual({
      johto: {
        "7": { replacement_block_id: 0x36, variant: "whirlpool" },
      },
    });
  });
});
