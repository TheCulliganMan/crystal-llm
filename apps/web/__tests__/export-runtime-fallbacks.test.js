const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { exportRuntimeAssets } = require("../scripts/export-runtime-fallbacks.js");

const makeDir = (dir) => fs.mkdirSync(dir, { recursive: true });
const writeFakeDisassembly = (root) => {
  makeDir(path.join(root, "constants"));
  makeDir(path.join(root, "data", "maps"));
  makeDir(path.join(root, "data", "moves"));
  makeDir(path.join(root, "data", "battle_anims"));
  makeDir(path.join(root, "data", "sprite_anims"));
  makeDir(path.join(root, "data", "text"));
  makeDir(path.join(root, "data", "phone", "text"));
  makeDir(path.join(root, "data", "phone"));
  makeDir(path.join(root, "data", "trainers"));
  makeDir(path.join(root, "data", "tilesets"));
  makeDir(path.join(root, "data", "collision"));
  makeDir(path.join(root, "data", "sprites"));
  makeDir(path.join(root, "engine", "events"));
  makeDir(path.join(root, "gfx", "battle_anims"));
  makeDir(path.join(root, "gfx", "tilesets"));
  makeDir(path.join(root, "maps"));

  fs.writeFileSync(path.join(root, "constants", "map_constants.asm"), "newgroup TEST_GROUP\nmap_const TEST_MAP, 10, 9 ; 1\n");
  fs.writeFileSync(path.join(root, "data", "maps", "maps.asm"), "map TEST_MAP, TILESET_JOHTO, TOWN, LANDMARK_TEST, MUSIC_NONE, FALSE, PALETTE_DAY, FISHGROUP_NONE\n");
  fs.writeFileSync(path.join(root, "data", "maps", "spawn_points.asm"), "spawn TEST_MAP, 4, 4\n");
  fs.writeFileSync(path.join(root, "data", "moves", "animations.asm"), "BattleAnimations::\n\tdw BattleAnim_Test\n\tassert_table_length NUM_BATTLE_ANIMS\n");
  fs.writeFileSync(path.join(root, "data", "battle_anims", "objects.asm"), "; BATTLE_ANIM_OBJ_HIT\nbattleanimobj 0, 0, BATTLE_ANIM_FRAMESET_HIT, BATTLE_ANIM_FUNC_NULL, PAL_BATTLE_OB_GRAY, BATTLE_ANIM_GFX_HIT\n");
  fs.writeFileSync(
    path.join(root, "data", "battle_anims", "object_gfx.asm"),
    "AnimObjGFX:\n\tanim_obj_gfx 0, AnimObj00GFX\n\tanim_obj_gfx 1, AnimObjHitGFX\n\tassert_table_length NUM_BATTLE_ANIM_GFX + 1\n"
  );
  fs.writeFileSync(path.join(root, "data", "battle_anims", "framesets.asm"), "dw .Frameset_Hit ; BATTLE_ANIM_FRAMESET_HIT\n.Frameset_Hit:\n\toamframe BATTLE_ANIM_OAMSET_HIT, 1\n\toamdelete\n");
  fs.writeFileSync(path.join(root, "data", "battle_anims", "oam.asm"), "battleanimoam 0, 1, .OAM_Hit ; BATTLE_ANIM_OAMSET_HIT\n.OAM_Hit:\n\tdbsprite 0, 0, 0, 0, 0, 0\n");
  fs.writeFileSync(
    path.join(root, "constants", "battle_anim_constants.asm"),
    [
      "BattleAnimObjects indexes",
      "const_def",
      "const BATTLE_ANIM_OBJ_HIT",
      "",
      "BattleAnimGFX indexes (see data/battle_anims/object_gfx.asm)",
      "const_def 1",
      "const BATTLE_ANIM_GFX_HIT ; 01",
      "DEF NUM_BATTLE_ANIM_GFX EQU const_value - 1",
      "",
    ].join("\n")
  );
  fs.writeFileSync(path.join(root, "gfx", "battle_anims.asm"), 'AnimObj00GFX: AnimObjHitGFX: INCBIN "gfx/battle_anims/hit.2bpp.lz"\n');
  fs.writeFileSync(path.join(root, "data", "sprite_anims", "oam.asm"), "spriteanimoam 0, .OAMData_Test ; SPRITE_ANIM_OAMSET_TEST\n.OAMData_Test:\n\tdb 1\n\tdbsprite 0, 0, 0, 0, 0, 0\n");
  fs.writeFileSync(path.join(root, "data", "sprite_anims", "framesets.asm"), "table_width 2, SpriteAnimFramesets\n\tdw .Frameset_Test\n\tassert_table_length NUM_SPRITE_ANIM_FRAMESETS\n.Frameset_Test:\n\toamframe SPRITE_ANIM_OAMSET_TEST, 1\n\toamend\n");
  fs.writeFileSync(path.join(root, "data", "sprite_anims", "objects.asm"), "; SPRITE_ANIM_OBJ_TEST\n\tdb SPRITE_ANIM_FRAMESET_TEST, SPRITE_ANIM_FUNC_NULL, SPRITE_ANIM_DICT_DEFAULT\n");
  fs.writeFileSync(path.join(root, "constants", "sprite_anim_constants.asm"), "const SPRITE_ANIM_FRAMESET_TEST\nDEF NUM_SPRITE_ANIM_FRAMESETS EQU const_value\n");
  fs.writeFileSync(path.join(root, "data", "maps", "blocks.asm"), 'TestMap_Blocks:\n\tINCBIN "maps/test_map.blk"\n');
  fs.writeFileSync(path.join(root, "maps", "test_map.blk"), Buffer.from([1, 2, 3, 4]));
  fs.writeFileSync(path.join(root, "constants", "phone_constants.asm"), "const PHONE_MOM\nDEF NUM_PHONE_CONTACTS EQU const_value\n");
  fs.writeFileSync(path.join(root, "data", "phone", "phone_contacts.asm"), "phone TRAINER_NONE, PHONECONTACT_MOM, N_A, ANYTIME, 0, ANYTIME, 0\n");
  fs.writeFileSync(path.join(root, "data", "phone", "non_trainer_names.asm"), '.MOM: db "MOM@"\n');
  fs.writeFileSync(path.join(root, "data", "phone", "permanent_numbers.asm"), "db PHONE_MOM\n\tdb -1\n");
  fs.writeFileSync(path.join(root, "data", "text", "battle.asm"), "JoeyAskNumber1Text:\n\ttext \"Battle text@\"\n\tline \"from asm@\"\n\tprompt\n");
  fs.writeFileSync(path.join(root, "data", "phone", "text", "extra.asm"), "JackNumberAcceptedText:\n\ttext \"Phone text@\"\n\tdone\n");
  makeDir(path.join(root, "data", "moves"));
  fs.writeFileSync(path.join(root, "data", "moves", "names.asm"), "MoveNames::\n\tlist_start\n\tli \"POUND\"\n");
  fs.writeFileSync(path.join(root, "engine", "events", "std_scripts.asm"), "InitializeEventsScript:\n\tsetevent EVENT_TEST\n\tsetflag ENGINE_TEST\n\tvariablesprite SPRITE_FUCHSIA_GYM_1, SPRITE_ROCKER\n\tendcallback\n");
  fs.writeFileSync(path.join(root, "constants", "misc_constants.asm"), "DEF MAX_COINS EQU 9999\nDEF COIN_CHUNK EQU 50\n");
  fs.writeFileSync(
    path.join(root, "constants", "pokemon_constants.asm"),
    [
      "const_def 1",
      "const UNOWN_A",
      "const UNOWN_B",
      "const UNOWN_C",
      "const UNOWN_D",
      "const UNOWN_E",
      "const UNOWN_F",
      "const UNOWN_G",
      "const UNOWN_H",
      "const UNOWN_I",
      "const UNOWN_J",
      "const UNOWN_K",
      "const UNOWN_L",
      "const UNOWN_M",
      "const UNOWN_N",
      "const UNOWN_O",
      "const UNOWN_P",
      "const UNOWN_Q",
      "const UNOWN_R",
      "const UNOWN_S",
      "const UNOWN_T",
      "const UNOWN_U",
      "const UNOWN_V",
      "const UNOWN_W",
      "const UNOWN_X",
      "const UNOWN_Y",
      "const UNOWN_Z",
      "DEF NUM_UNOWN EQU const_value - 1",
      "",
    ].join("\n")
  );
  fs.writeFileSync(
    path.join(root, "constants", "ram_constants.asm"),
    [
      "const_def",
      "const ZEPHYRBADGE",
      "const HIVEBADGE",
      "DEF NUM_JOHTO_BADGES EQU const_value",
      "const_def",
      "const BOULDERBADGE",
      "DEF NUM_KANTO_BADGES EQU const_value",
      "DEF NUM_BADGES EQU NUM_JOHTO_BADGES + NUM_KANTO_BADGES",
      "",
    ].join("\n")
  );
  fs.writeFileSync(path.join(root, "constants", "battle_constants.asm"), "DEF EGG_LEVEL EQU 5\n");
  fs.writeFileSync(path.join(root, "maps", "TestMap.asm"), "DEF MAP_COIN_GIFT EQU COIN_CHUNK * 2\n");
  fs.writeFileSync(path.join(root, "constants", "trainer_constants.asm"), "");
  fs.writeFileSync(path.join(root, "data", "trainers", "class_names.asm"), "");
  fs.writeFileSync(path.join(root, "constants", "sprite_constants.asm"), "const SPRITE_NONE\nconst SPRITE_CHRIS\nDEF NUM_OVERWORLD_SPRITES EQU const_value\n");
  fs.writeFileSync(path.join(root, "data", "sprites", "sprites.asm"), "overworld_sprite SPRITE_CHRIS, 0, 0, PAL_OW_RED\n");
  fs.writeFileSync(path.join(root, "data", "tilesets", "johto_metatiles.bin"), Buffer.from([0, 1, 2, 3]));
  fs.writeFileSync(path.join(root, "data", "tilesets", "players_room_metatiles.bin"), Buffer.from([0, 1, 2, 3]));
  fs.writeFileSync(path.join(root, "data", "tilesets", "johto_collision.asm"), "tilecoll FLOOR, FLOOR, FLOOR, FLOOR ; 00\n");
  fs.writeFileSync(path.join(root, "data", "tilesets", "players_room_collision.asm"), "tilecoll FLOOR, FLOOR, FLOOR, FLOOR ; 00\n");
  fs.writeFileSync(path.join(root, "gfx", "tilesets", "johto_palette_map.asm"), "tilepal 0, GRAY, RED, GREEN, WATER, YELLOW, BROWN, ROOF, TEXT\n");
  fs.writeFileSync(path.join(root, "gfx", "tilesets", "players_room_palette_map.asm"), "tilepal 0, GRAY, RED, GREEN, WATER, YELLOW, BROWN, ROOF, TEXT\n");
  fs.writeFileSync(path.join(root, "data", "collision", "collision_permissions.asm"), "db LAND_TILE ; FLOOR\n");
  fs.writeFileSync(path.join(root, "data", "collision", "collision_stdscripts.asm"), "std_collision DEFAULT, .Default\n.Default:\n\tdb 0\n");
};

const writeCommittedRuntimeAssets = (outDir) => {
  makeDir(path.join(outDir, "tilesets"));
  makeDir(path.join(outDir, "collision"));
  fs.writeFileSync(path.join(outDir, "runtime_map_metadata.json"), "{}\n");
  fs.writeFileSync(path.join(outDir, "runtime_spawn_points.json"), "{}\n");
  fs.writeFileSync(path.join(outDir, "battle_animation_table.json"), "[]\n");
  fs.writeFileSync(
    path.join(outDir, "battle_anim_bundle.json"),
    "{\"objects\":{},\"framesets\":{},\"oam_sets\":{},\"gfx_table\":{},\"gfx_sources\":{}}\n"
  );
  fs.writeFileSync(path.join(outDir, "asm_text.json"), "{\"DummyText\":\"Hello\"}\n");
  fs.writeFileSync(path.join(outDir, "move_names.json"), "[\"POUND\"]\n");
  fs.writeFileSync(
    path.join(outDir, "sprite_anim_bundle.json"),
    "{\"oam_sets\":{},\"framesets\":{},\"objects\":{}}\n"
  );
  fs.writeFileSync(path.join(outDir, "map_blocks.json"), "{}\n");
  fs.writeFileSync(path.join(outDir, "phone_contacts.json"), "{}\n");
  fs.writeFileSync(path.join(outDir, "permanent_phone_numbers.json"), "[]\n");
  fs.writeFileSync(path.join(outDir, "initialize_events.json"), "{}\n");
  fs.writeFileSync(path.join(outDir, "story_event_script_constants.json"), "{\"global\":{\"MAX_COINS\":9999},\"maps\":{}}\n");
  fs.writeFileSync(path.join(outDir, "sprite_palette_defaults.json"), "{}\n");
  fs.writeFileSync(path.join(outDir, "tilesets", "johto.json"), "{\"00\":[\"FLOOR\",\"FLOOR\",\"FLOOR\",\"FLOOR\"]}\n");
  fs.writeFileSync(path.join(outDir, "tilesets", "johto_metatiles.bin"), Buffer.from([0, 1, 2, 3]));
  fs.writeFileSync(path.join(outDir, "tilesets", "johto_palette_map.json"), "[0,1,2,3]\n");
  fs.writeFileSync(path.join(outDir, "tilesets", "players_room.json"), "{\"00\":[\"FLOOR\",\"FLOOR\",\"FLOOR\",\"FLOOR\"]}\n");
  fs.writeFileSync(path.join(outDir, "tilesets", "players_room_metatiles.bin"), Buffer.from([0, 1, 2, 3]));
  fs.writeFileSync(path.join(outDir, "tilesets", "players_room_palette_map.json"), "[0,1,2,3]\n");
  fs.writeFileSync(path.join(outDir, "collision", "collision_permissions.json"), "[]\n");
  fs.writeFileSync(path.join(outDir, "collision", "collision_stdscripts.json"), "{}\n");
};

describe("export-runtime-fallbacks", () => {
  test("accepts committed runtime assets when disassembly is missing", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-assets-"));

    try {
      const outDir = path.join(tempRoot, "assets", "data");
      writeCommittedRuntimeAssets(outDir);

      expect(() =>
        exportRuntimeAssets({
          projectRoot: tempRoot,
          disassemblyRoot: path.join(tempRoot, "missing_disassembly"),
          outDir,
        })
      ).not.toThrow();
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("fails fast when neither disassembly nor committed runtime assets are available", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-assets-missing-"));

    try {
      const outDir = path.join(tempRoot, "assets", "data");
      makeDir(outDir);

      expect(() =>
        exportRuntimeAssets({
          projectRoot: tempRoot,
          disassemblyRoot: path.join(tempRoot, "missing_disassembly"),
          outDir,
        })
      ).toThrow(/Missing generated runtime assets/);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("fails fast when committed battle animation runtime bundle is missing", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-assets-partial-"));

    try {
      const outDir = path.join(tempRoot, "assets", "data");
      writeCommittedRuntimeAssets(outDir);
      fs.rmSync(path.join(outDir, "battle_anim_bundle.json"));

      expect(() =>
        exportRuntimeAssets({
          projectRoot: tempRoot,
          disassemblyRoot: path.join(tempRoot, "missing_disassembly"),
          outDir,
        })
      ).toThrow(/battle_anim_bundle\.json/);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("exports story event runtime datasets from disassembly", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-assets-export-"));

    try {
      const outDir = path.join(tempRoot, "assets", "data");
      const disassemblyRoot = path.join(tempRoot, "pokecrystal_disassembly");
      writeFakeDisassembly(disassemblyRoot);

      exportRuntimeAssets({
        projectRoot: tempRoot,
        disassemblyRoot,
        outDir,
      });

      expect(JSON.parse(fs.readFileSync(path.join(outDir, "permanent_phone_numbers.json"), "utf8"))).toEqual([
        "PHONE_MOM",
      ]);
      expect(JSON.parse(fs.readFileSync(path.join(outDir, "initialize_events.json"), "utf8"))).toEqual({
        eventFlags: ["EVENT_TEST"],
        engineFlags: ["ENGINE_TEST"],
        variableSprites: {
          SPRITE_FUCHSIA_GYM_1: "SPRITE_ROCKER",
        },
      });
      expect(JSON.parse(fs.readFileSync(path.join(outDir, "story_event_script_constants.json"), "utf8"))).toEqual({
        global: {
          MAX_COINS: 9999,
          COIN_CHUNK: 50,
          UNOWN_A: 1,
          UNOWN_B: 2,
          UNOWN_C: 3,
          UNOWN_D: 4,
          UNOWN_E: 5,
          UNOWN_F: 6,
          UNOWN_G: 7,
          UNOWN_H: 8,
          UNOWN_I: 9,
          UNOWN_J: 10,
          UNOWN_K: 11,
          UNOWN_L: 12,
          UNOWN_M: 13,
          UNOWN_N: 14,
          UNOWN_O: 15,
          UNOWN_P: 16,
          UNOWN_Q: 17,
          UNOWN_R: 18,
          UNOWN_S: 19,
          UNOWN_T: 20,
          UNOWN_U: 21,
          UNOWN_V: 22,
          UNOWN_W: 23,
          UNOWN_X: 24,
          UNOWN_Y: 25,
          UNOWN_Z: 26,
          NUM_UNOWN: 26,
          EGG_LEVEL: 5,
          ZEPHYRBADGE: 0,
          HIVEBADGE: 1,
          NUM_JOHTO_BADGES: 2,
          BOULDERBADGE: 0,
          NUM_KANTO_BADGES: 1,
          NUM_BADGES: 3,
        },
        maps: {
          TestMap: {
            MAP_COIN_GIFT: 100,
          },
        },
      });
      expect(JSON.parse(fs.readFileSync(path.join(outDir, "collision", "collision_stdscripts.json"), "utf8"))).toEqual({
        DEFAULT: "Default",
      });
      expect(JSON.parse(fs.readFileSync(path.join(outDir, "asm_text.json"), "utf8"))).toMatchObject({
        JoeyAskNumber1Text: "Battle text\nfrom asm",
        JackNumberAcceptedText: "Phone text",
      });
      expect(JSON.parse(fs.readFileSync(path.join(outDir, "move_names.json"), "utf8"))).toEqual(["POUND"]);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("exports battle animation runtime bundle from disassembly", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-assets-battle-bundle-"));

    try {
      const outDir = path.join(tempRoot, "assets", "data");
      const disassemblyRoot = path.join(tempRoot, "pokecrystal_disassembly");
      writeFakeDisassembly(disassemblyRoot);

      exportRuntimeAssets({
        projectRoot: tempRoot,
        disassemblyRoot,
        outDir,
      });

      const bundle = JSON.parse(fs.readFileSync(path.join(outDir, "battle_anim_bundle.json"), "utf8"));
      expect(bundle).toMatchObject({
        objects: {
          BATTLE_ANIM_OBJ_HIT: {
            object_id: "BATTLE_ANIM_OBJ_HIT",
            function: null,
            frameset: "BATTLE_ANIM_FRAMESET_HIT",
            palette: "PAL_BATTLE_OB_GRAY",
            gfx_id: "BATTLE_ANIM_GFX_HIT",
          },
        },
        framesets: {
          BATTLE_ANIM_FRAMESET_HIT: [
            {
              command: "frame",
              oam_set: "BATTLE_ANIM_OAMSET_HIT",
              duration: 1,
              xflip: false,
              yflip: false,
            },
            {
              command: "delete",
              oam_set: null,
              duration: 0,
              xflip: false,
              yflip: false,
            },
          ],
        },
        oam_sets: {
          BATTLE_ANIM_OAMSET_HIT: {
            name: "BATTLE_ANIM_OAMSET_HIT",
            tile_offset: 0,
            entries: [{ x: 0, y: 0, tile_id: 0, xflip: false, yflip: false, obp: 0 }],
          },
        },
        gfx_table: {
          BATTLE_ANIM_GFX_HIT: [1, "AnimObjHitGFX"],
          BATTLE_ANIM_GFX_0: [0, "AnimObj00GFX"],
        },
        gfx_sources: {
          AnimObj00GFX: "gfx/battle_anims/hit.2bpp.lz",
          AnimObjHitGFX: "gfx/battle_anims/hit.2bpp.lz",
        },
      });
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("exports battle animation bundle gfx aliases from shared INCBIN lines", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-assets-battle-bundle-"));

    try {
      const outDir = path.join(tempRoot, "assets", "data");
      const disassemblyRoot = path.join(tempRoot, "pokecrystal_disassembly");
      writeFakeDisassembly(disassemblyRoot);

      exportRuntimeAssets({
        projectRoot: tempRoot,
        disassemblyRoot,
        outDir,
      });

      const bundle = JSON.parse(
        fs.readFileSync(path.join(outDir, "battle_anim_bundle.json"), "utf8")
      );
      expect(bundle.gfx_table.BATTLE_ANIM_GFX_HIT).toEqual([1, "AnimObjHitGFX"]);
      expect(bundle.gfx_sources.AnimObj00GFX).toBe("gfx/battle_anims/hit.2bpp.lz");
      expect(bundle.gfx_sources.AnimObjHitGFX).toBe("gfx/battle_anims/hit.2bpp.lz");
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("maps battle animation gfx constants by explicit numeric id instead of positional order", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-assets-battle-gfx-ids-"));

    try {
      const outDir = path.join(tempRoot, "assets", "data");
      const disassemblyRoot = path.join(tempRoot, "pokecrystal_disassembly");
      writeFakeDisassembly(disassemblyRoot);

      fs.writeFileSync(
        path.join(disassemblyRoot, "constants", "battle_anim_constants.asm"),
        [
          "BattleAnimObjects indexes",
          "const_def",
          "const BATTLE_ANIM_OBJ_HIT",
          "",
          "BattleAnimGFX indexes (see data/battle_anims/object_gfx.asm)",
          "const_def 1",
          "const BATTLE_ANIM_GFX_HIT ; 01",
          "const_skip ; 02",
          "const BATTLE_ANIM_GFX_FIRE ; 03",
          "DEF NUM_BATTLE_ANIM_GFX EQU const_value - 1",
          "",
        ].join("\n")
      );
      fs.writeFileSync(
        path.join(disassemblyRoot, "data", "battle_anims", "object_gfx.asm"),
        [
          "AnimObjGFX:",
          "\ttable_width 4",
          "\tanim_obj_gfx 0, AnimObj00GFX",
          "\tanim_obj_gfx 1, AnimObjHitGFX",
          "\tanim_obj_gfx 9, AnimObjUnusedGFX",
          "\tanim_obj_gfx 6, AnimObjFireGFX",
          "\tassert_table_length NUM_BATTLE_ANIM_GFX + 1",
          "",
        ].join("\n")
      );

      exportRuntimeAssets({
        projectRoot: tempRoot,
        disassemblyRoot,
        outDir,
      });

      const bundle = JSON.parse(
        fs.readFileSync(path.join(outDir, "battle_anim_bundle.json"), "utf8")
      );
      expect(bundle.gfx_table).toMatchObject({
        BATTLE_ANIM_GFX_0: [0, "AnimObj00GFX"],
        BATTLE_ANIM_GFX_HIT: [1, "AnimObjHitGFX"],
        BATTLE_ANIM_GFX_FIRE: [6, "AnimObjFireGFX"],
      });
      expect(bundle.gfx_table.BATTLE_ANIM_GFX_FIRE).not.toEqual([9, "AnimObjUnusedGFX"]);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("exports compact db battle animation oam entries into the runtime bundle", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-assets-battle-oam-"));

    try {
      const outDir = path.join(tempRoot, "assets", "data");
      const disassemblyRoot = path.join(tempRoot, "pokecrystal_disassembly");
      writeFakeDisassembly(disassemblyRoot);

      fs.writeFileSync(
        path.join(disassemblyRoot, "constants", "battle_anim_constants.asm"),
        [
          "; BattleAnimObjects indexes (see data/battle_anims/objects.asm)",
          "const_def",
          "const BATTLE_ANIM_OBJ_HIT",
          "",
          "BattleAnimGFX indexes",
          "const_def 1",
          "const BATTLE_ANIM_GFX_HIT",
          "DEF NUM_BATTLE_ANIM_GFX EQU const_value - 1",
          "",
        ].join("\n")
      );
      fs.writeFileSync(
        path.join(disassemblyRoot, "data", "battle_anims", "object_gfx.asm"),
        [
          "AnimObjGFX:",
          "\ttable_width 4",
          "\tanim_obj_gfx 0, AnimObj00GFX",
          "\tanim_obj_gfx 1, AnimObjHitGFX",
          "\tassert_table_length NUM_BATTLE_ANIM_GFX + 1",
          "",
        ].join("\n")
      );
      fs.writeFileSync(
        path.join(disassemblyRoot, "data", "battle_anims", "oam.asm"),
        [
          "battleanimoam 3, 1, .OAMData_Hit ; BATTLE_ANIM_OAMSET_HIT",
          ".OAMData_Hit:",
          "\tdb 1",
          "\tdb -2, 5, $07, OAM_XFLIP | OAM_YFLIP | OAM_PAL1",
          "",
        ].join("\n")
      );

      exportRuntimeAssets({
        projectRoot: tempRoot,
        disassemblyRoot,
        outDir,
      });

      const bundle = JSON.parse(
        fs.readFileSync(path.join(outDir, "battle_anim_bundle.json"), "utf8")
      );
      expect(bundle.oam_sets.BATTLE_ANIM_OAMSET_HIT).toEqual({
        name: "BATTLE_ANIM_OAMSET_HIT",
        tile_offset: 3,
        entries: [
          {
            x: -2,
            y: 5,
            tile_id: 7,
            xflip: true,
            yflip: true,
            obp: 1,
          },
        ],
      });
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });
});
