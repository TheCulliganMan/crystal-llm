const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { preparePublic } = require("../scripts/prepare-public.js");
const { PNG } = require("pngjs");

const makeDir = (dir) => fs.mkdirSync(dir, { recursive: true });
const writeSolidPng = (filePath) => {
  const png = new PNG({ width: 8, height: 8 });
  for (let offset = 0; offset < png.data.length; offset += 4) {
    png.data[offset] = 0;
    png.data[offset + 1] = 0;
    png.data[offset + 2] = 0;
    png.data[offset + 3] = 255;
  }
  fs.writeFileSync(filePath, PNG.sync.write(png));
};
const writeFakeDisassembly = (disassemblySource) => {
  makeDir(path.join(disassemblySource, "constants"));
  makeDir(path.join(disassemblySource, "data", "maps"));
  makeDir(path.join(disassemblySource, "data", "moves"));
  makeDir(path.join(disassemblySource, "data", "battle_anims"));
  makeDir(path.join(disassemblySource, "data", "sprite_anims"));
  makeDir(path.join(disassemblySource, "data", "text"));
  makeDir(path.join(disassemblySource, "data", "phone", "text"));
  makeDir(path.join(disassemblySource, "data", "phone"));
  makeDir(path.join(disassemblySource, "data", "pokemon"));
  makeDir(path.join(disassemblySource, "data", "trainers"));
  makeDir(path.join(disassemblySource, "data", "tilesets"));
  makeDir(path.join(disassemblySource, "data", "collision"));
  makeDir(path.join(disassemblySource, "data", "sprites"));
  makeDir(path.join(disassemblySource, "engine", "events"));
  makeDir(path.join(disassemblySource, "gfx"));
  makeDir(path.join(disassemblySource, "maps"));

  fs.writeFileSync(
    path.join(disassemblySource, "constants", "map_constants.asm"),
    "newgroup TEST_GROUP\nmap_const TEST_MAP, 10, 9 ; 1\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "maps", "maps.asm"),
    "map TEST_MAP, TILESET_JOHTO, TOWN, LANDMARK_TEST, MUSIC_NONE, FALSE, PALETTE_DAY, FISHGROUP_NONE\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "maps", "spawn_points.asm"),
    "spawn TEST_MAP, 4, 4\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "moves", "animations.asm"),
    "BattleAnimations::\n\tdw BattleAnim_Test\n\tassert_table_length NUM_BATTLE_ANIMS\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "battle_anims", "objects.asm"),
    "; BATTLE_ANIM_OBJ_HIT\nbattleanimobj 0, 0, BATTLE_ANIM_FRAMESET_HIT, BATTLE_ANIM_FUNC_NULL, PAL_BATTLE_OB_GRAY, BATTLE_ANIM_GFX_HIT\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "battle_anims", "object_gfx.asm"),
    "anim_obj_gfx 0, ANIM_GFX_0\nanim_obj_gfx 1, ANIM_GFX_HIT\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "battle_anims", "framesets.asm"),
    "dw .Frameset_Hit ; BATTLE_ANIM_FRAMESET_HIT\n.Frameset_Hit:\n\toamframe .OAM_Hit, 1\n\toamdelete\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "battle_anims", "oam.asm"),
    ".OAM_Hit:\n\tdb 1\n\tdb 0, 0, 0, 0\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "constants", "battle_anim_constants.asm"),
    [
      "BattleAnimObjects indexes",
      "const_def",
      "const BATTLE_ANIM_OBJ_HIT",
      "",
      "BattleAnimGFX indexes (see data/battle_anims/object_gfx.asm)",
      "const_def 1",
      "const BATTLE_ANIM_GFX_HIT",
      "DEF NUM_BATTLE_ANIM_GFX EQU const_value - 1",
      "",
    ].join("\n")
  );
  fs.writeFileSync(
    path.join(disassemblySource, "gfx", "battle_anims.asm"),
    'AnimObjHitGFX: INCBIN "gfx/battle_anims/hit.2bpp.lz"\n'
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "sprite_anims", "oam.asm"),
    "spriteanimoam 0, .OAMData_Test ; SPRITE_ANIM_OAMSET_TEST\n.OAMData_Test:\n\tdb 1\n\tdbsprite 0, 0, 0, 0, 0, 0\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "sprite_anims", "framesets.asm"),
    "table_width 2, SpriteAnimFramesets\n\tdw .Frameset_Test\n\tassert_table_length NUM_SPRITE_ANIM_FRAMESETS\n.Frameset_Test:\n\toamframe SPRITE_ANIM_OAMSET_TEST, 1\n\toamend\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "sprite_anims", "objects.asm"),
    "; SPRITE_ANIM_OBJ_TEST\n\tdb SPRITE_ANIM_FRAMESET_TEST, SPRITE_ANIM_FUNC_NULL, SPRITE_ANIM_DICT_DEFAULT\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "constants", "sprite_anim_constants.asm"),
    "const SPRITE_ANIM_FRAMESET_TEST\nDEF NUM_SPRITE_ANIM_FRAMESETS EQU const_value\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "maps", "blocks.asm"),
    'TestMap_Blocks:\n\tINCBIN "maps/test_map.blk"\n'
  );
  fs.writeFileSync(path.join(disassemblySource, "maps", "test_map.blk"), Buffer.from([1, 2, 3, 4]));
  fs.writeFileSync(
    path.join(disassemblySource, "constants", "phone_constants.asm"),
    "const PHONE_00\nconst PHONE_MOM\nDEF NUM_PHONE_CONTACTS EQU const_value\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "phone", "phone_contacts.asm"),
    "phone TRAINER_NONE, PHONECONTACT_MOM, N_A, ANYTIME, 0, ANYTIME, 0\nphone TRAINER_NONE, PHONECONTACT_MOM, N_A, ANYTIME, 0, ANYTIME, 0\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "phone", "non_trainer_names.asm"),
    '.MOM: db "MOM@"\n'
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "phone", "permanent_numbers.asm"),
    "db PHONE_MOM\n\tdb -1\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "pokemon", "cries.asm"),
    "MACRO mon_cry\nENDM\n\tmon_cry CRY_BULBASAUR, 128, 129 ; BULBASAUR\n\tmon_cry CRY_PIDGEOTTO, 17, 383 ; PIDGEOT\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "text", "battle.asm"),
    "JoeyAskNumber1Text:\n\ttext \"Battle text@\"\n\tprompt\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "phone", "text", "extra.asm"),
    "JackNumberAcceptedText:\n\ttext \"Phone text@\"\n\tdone\n"
  );
  makeDir(path.join(disassemblySource, "data", "moves"));
  fs.writeFileSync(
    path.join(disassemblySource, "data", "moves", "names.asm"),
    "MoveNames::\n\tlist_start\n\tli \"POUND\"\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "engine", "events", "std_scripts.asm"),
    "InitializeEventsScript:\n\tsetevent EVENT_TEST\n\tsetflag ENGINE_TEST\n\tvariablesprite SPRITE_FUCHSIA_GYM_1, SPRITE_ROCKER\n\tendcallback\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "constants", "misc_constants.asm"),
    "DEF MAX_COINS EQU 9999\nDEF COIN_CHUNK EQU 50\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "maps", "TestMap.asm"),
    "DEF MAP_COIN_GIFT EQU COIN_CHUNK * 2\n"
  );
  fs.writeFileSync(path.join(disassemblySource, "constants", "trainer_constants.asm"), "");
  fs.writeFileSync(path.join(disassemblySource, "data", "trainers", "class_names.asm"), "");
  fs.writeFileSync(
    path.join(disassemblySource, "constants", "sprite_constants.asm"),
    "const SPRITE_NONE\nconst SPRITE_CHRIS\nDEF NUM_OVERWORLD_SPRITES EQU const_value\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "sprites", "sprites.asm"),
    "overworld_sprite SPRITE_CHRIS, 0, 0, PAL_OW_RED\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "tilesets", "johto_metatiles.bin"),
    Buffer.from([0, 1, 2, 3])
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "tilesets", "players_room_metatiles.bin"),
    Buffer.from([0, 1, 2, 3])
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "tilesets", "johto_collision.asm"),
    "tilecoll FLOOR, FLOOR, FLOOR, FLOOR ; 00\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "tilesets", "players_room_collision.asm"),
    "tilecoll FLOOR, FLOOR, FLOOR, FLOOR ; 00\n"
  );
  makeDir(path.join(disassemblySource, "gfx", "tilesets"));
  fs.writeFileSync(
    path.join(disassemblySource, "gfx", "tilesets", "johto_palette_map.asm"),
    "tilepal 0, GRAY, RED, GREEN, WATER, YELLOW, BROWN, ROOF, TEXT\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "gfx", "tilesets", "players_room_palette_map.asm"),
    "tilepal 0, GRAY, RED, GREEN, WATER, YELLOW, BROWN, ROOF, TEXT\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "collision", "collision_permissions.asm"),
    "db LAND_TILE ; FLOOR\n"
  );
  fs.writeFileSync(
    path.join(disassemblySource, "data", "collision", "collision_stdscripts.asm"),
    "std_collision DEFAULT, .Default\n.Default:\n\tdb 0\n"
  );
};
const writeCommittedRuntimeAssets = (sourceDataDir) => {
  makeDir(path.join(sourceDataDir, "tilesets"));
  makeDir(path.join(sourceDataDir, "collision"));
  fs.writeFileSync(path.join(sourceDataDir, "runtime_map_metadata.json"), "{}\n");
  fs.writeFileSync(path.join(sourceDataDir, "runtime_spawn_points.json"), "{}\n");
  fs.writeFileSync(path.join(sourceDataDir, "battle_animation_table.json"), "[]\n");
  fs.writeFileSync(
    path.join(sourceDataDir, "battle_anim_bundle.json"),
    "{\"objects\":{},\"framesets\":{},\"oam_sets\":{},\"gfx_table\":{},\"gfx_sources\":{}}\n"
  );
  fs.writeFileSync(path.join(sourceDataDir, "asm_text.json"), "{\"DummyText\":\"Hello\"}\n");
  fs.writeFileSync(path.join(sourceDataDir, "move_names.json"), "[\"POUND\"]\n");
  fs.writeFileSync(
    path.join(sourceDataDir, "sprite_anim_bundle.json"),
    "{\"oam_sets\":{},\"framesets\":{},\"objects\":{}}\n"
  );
  fs.writeFileSync(path.join(sourceDataDir, "map_blocks.json"), "{}\n");
  fs.writeFileSync(path.join(sourceDataDir, "phone_contacts.json"), "{}\n");
  fs.writeFileSync(path.join(sourceDataDir, "pokemon_cries.json"), "{}\n");
  fs.writeFileSync(path.join(sourceDataDir, "permanent_phone_numbers.json"), "[]\n");
  fs.writeFileSync(path.join(sourceDataDir, "initialize_events.json"), "{}\n");
  fs.writeFileSync(path.join(sourceDataDir, "story_event_script_constants.json"), "{\"global\":{\"MAX_COINS\":9999},\"maps\":{}}\n");
  fs.writeFileSync(path.join(sourceDataDir, "sprite_palette_defaults.json"), "{}\n");
  fs.writeFileSync(path.join(sourceDataDir, "tilesets", "johto.json"), "{\"00\":[\"FLOOR\",\"FLOOR\",\"FLOOR\",\"FLOOR\"]}\n");
  fs.writeFileSync(path.join(sourceDataDir, "tilesets", "johto_metatiles.bin"), Buffer.from([0, 1, 2, 3]));
  fs.writeFileSync(path.join(sourceDataDir, "tilesets", "johto_palette_map.json"), "[0,1,2,3]\n");
  fs.writeFileSync(path.join(sourceDataDir, "tilesets", "players_room.json"), "{\"00\":[\"FLOOR\",\"FLOOR\",\"FLOOR\",\"FLOOR\"]}\n");
  fs.writeFileSync(path.join(sourceDataDir, "tilesets", "players_room_metatiles.bin"), Buffer.from([0, 1, 2, 3]));
  fs.writeFileSync(path.join(sourceDataDir, "tilesets", "players_room_palette_map.json"), "[0,1,2,3]\n");
  fs.writeFileSync(path.join(sourceDataDir, "collision", "collision_permissions.json"), "[]\n");
  fs.writeFileSync(path.join(sourceDataDir, "collision", "collision_stdscripts.json"), "{}\n");
};

describe("prepare-public", () => {
  test("exports runtime assets without creating a public disassembly mirror", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-public-"));

    try {
      const publicDir = path.join(tempRoot, "public");
      const assetsSource = path.join(tempRoot, "assets");
      const disassemblySource = path.join(tempRoot, "pokecrystal_disassembly");

      makeDir(publicDir);
      makeDir(assetsSource);
      makeDir(disassemblySource);
      writeFakeDisassembly(disassemblySource);

      fs.writeFileSync(path.join(assetsSource, "assets.txt"), "assets");
      fs.writeFileSync(path.join(disassemblySource, "disassembly.txt"), "disassembly");

      preparePublic({
        projectRoot: tempRoot,
        assetsSource,
        disassemblySource,
      });

      expect(fs.existsSync(path.join(publicDir, "assets"))).toBe(false);
      expect(fs.existsSync(path.join(publicDir, "disassembly"))).toBe(false);
      expect(fs.existsSync(path.join(assetsSource, "data", "runtime_map_metadata.json"))).toBe(true);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("does not create or refresh public assets mirror", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-public-assets-"));

    try {
      const publicDir = path.join(tempRoot, "public");
      const assetsSource = path.join(tempRoot, "assets");
      const disassemblySource = path.join(tempRoot, "pokecrystal_disassembly");
      const sourceDataDir = path.join(assetsSource, "data");
      const assetsTarget = path.join(publicDir, "assets");

      makeDir(publicDir);
      makeDir(assetsSource);
      makeDir(disassemblySource);
      writeFakeDisassembly(disassemblySource);
      makeDir(sourceDataDir);

      writeCommittedRuntimeAssets(sourceDataDir);

      preparePublic({
        projectRoot: tempRoot,
        assetsSource,
        disassemblySource,
      });

      expect(fs.existsSync(assetsTarget)).toBe(false);
      expect(fs.existsSync(path.join(publicDir, "disassembly"))).toBe(false);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("removes any stale public assets mirror", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-public-runtime-sync-"));

    try {
      const publicDir = path.join(tempRoot, "public");
      const assetsSource = path.join(tempRoot, "assets");
      const disassemblySource = path.join(tempRoot, "missing_disassembly");
      const sourceDataDir = path.join(assetsSource, "data");
      const publicAssetsDataDir = path.join(publicDir, "assets", "data");

      makeDir(publicAssetsDataDir);
      makeDir(assetsSource);
      makeDir(sourceDataDir);
      writeCommittedRuntimeAssets(sourceDataDir);

      fs.writeFileSync(path.join(publicAssetsDataDir, "stale.json"), "{\"stale\":true}\n");

      preparePublic({
        projectRoot: tempRoot,
        assetsSource,
        disassemblySource,
      });

      expect(fs.existsSync(path.join(publicDir, "assets"))).toBe(false);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("uses committed runtime assets when disassembly is unavailable", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-public-runtime-"));

    try {
      const publicDir = path.join(tempRoot, "public");
      const assetsSource = path.join(tempRoot, "assets");
      const assetsTarget = path.join(publicDir, "assets");
      const disassemblySource = path.join(tempRoot, "missing_disassembly");
      const sourceDataDir = path.join(assetsSource, "data");

      makeDir(publicDir);
      makeDir(assetsSource);
      makeDir(sourceDataDir);

      writeCommittedRuntimeAssets(sourceDataDir);

      preparePublic({
        projectRoot: tempRoot,
        assetsSource,
        disassemblySource,
      });

      expect(fs.existsSync(assetsTarget)).toBe(false);
      expect(fs.existsSync(path.join(sourceDataDir, "runtime_map_metadata.json"))).toBe(true);
      expect(fs.existsSync(path.join(publicDir, "disassembly"))).toBe(false);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("generates Unown puzzle runtime assets from PNG sources", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-public-unown-"));

    try {
      const publicDir = path.join(tempRoot, "public");
      const assetsSource = path.join(tempRoot, "assets");
      const sourceDataDir = path.join(assetsSource, "data");
      const unownGfxDir = path.join(assetsSource, "gfx", "unown_puzzle");
      const disassemblySource = path.join(tempRoot, "missing_disassembly");

      makeDir(publicDir);
      makeDir(sourceDataDir);
      makeDir(unownGfxDir);
      writeCommittedRuntimeAssets(sourceDataDir);
      writeSolidPng(path.join(unownGfxDir, "cursor.png"));

      preparePublic({
        projectRoot: tempRoot,
        assetsSource,
        disassemblySource,
      });

      expect(fs.existsSync(path.join(unownGfxDir, "cursor.2bpp"))).toBe(true);
      expect(fs.readFileSync(path.join(unownGfxDir, "cursor.2bpp"))).toHaveLength(16);
      expect(fs.existsSync(path.join(assetsSource, "data", "unown_puzzles", "coordinates.json"))).toBe(true);
      expect(fs.existsSync(path.join(assetsSource, "data", "unown_puzzles", "layouts.json"))).toBe(true);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("generates Pokemon cry runtime lookup data from disassembly sources", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-public-cries-"));

    try {
      const assetsSource = path.join(tempRoot, "assets");
      const disassemblySource = path.join(tempRoot, "pokecrystal_disassembly");
      const sourceDataDir = path.join(assetsSource, "data");

      makeDir(assetsSource);
      makeDir(sourceDataDir);
      writeCommittedRuntimeAssets(sourceDataDir);
      writeFakeDisassembly(disassemblySource);

      preparePublic({
        projectRoot: tempRoot,
        assetsSource,
        disassemblySource,
      });

      const cries = JSON.parse(
        fs.readFileSync(path.join(sourceDataDir, "pokemon_cries.json"), "utf8")
      );
      expect(cries.BULBASAUR).toEqual({ cry: "CRY_BULBASAUR", pitch: 128, length: 129 });
      expect(cries.PIDGEOT).toEqual({ cry: "CRY_PIDGEOTTO", pitch: 17, length: 383 });
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("does not require disassembly exporter inputs when committed runtime assets already exist", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "poke-public-asset-only-"));

    try {
      const publicDir = path.join(tempRoot, "public");
      const assetsSource = path.join(tempRoot, "assets");
      const disassemblySource = path.join(tempRoot, "pokecrystal_disassembly");
      const sourceDataDir = path.join(assetsSource, "data");

      makeDir(publicDir);
      makeDir(assetsSource);
      makeDir(disassemblySource);
      makeDir(sourceDataDir);
      writeCommittedRuntimeAssets(sourceDataDir);

      expect(() =>
        preparePublic({
          projectRoot: tempRoot,
          assetsSource,
          disassemblySource,
        })
      ).not.toThrow();

      expect(fs.existsSync(path.join(sourceDataDir, "runtime_map_metadata.json"))).toBe(true);
      expect(fs.existsSync(path.join(publicDir, "disassembly"))).toBe(false);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });
});
