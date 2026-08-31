import fs from "fs";
import path from "path";
import {
  RUNTIME_PRESENTATION_ENTRYPOINTS,
  analyzeRuntimePresentationControlFlow,
  analyzeRuntimeTitlePresentationEmission,
  assertRuntimePresentationProgram,
  buildRuntimeTitlePresentationProgram,
  requireClosedRuntimePresentationControlFlow,
} from "./export-runtime-title-screen";

const repositoryRoot = path.resolve(__dirname, "../../../..");
const disassemblyRoot = path.join(repositoryRoot, "vendor/pokecrystal");
const introPath = "engine/menus/intro_menu.asm";
const mainMenuPath = "engine/menus/main_menu.asm";
const titlePath = "engine/movie/title.asm";
const splashPath = "engine/movie/splash.asm";
const spriteObjectsPath = "data/sprite_anims/objects.asm";
const spriteFunctionsPath = "engine/sprite_anims/functions.asm";
const spriteFramesetsPath = "data/sprite_anims/framesets.asm";
const spriteOamPath = "data/sprite_anims/oam.asm";
const spriteCorePath = "engine/sprite_anims/core.asm";
const wramPath = "ram/wram.asm";
const splashPalettePath = "gfx/splash/ditto_fade.pal";
const delayPath = "home/delay.asm";
const savePath = "engine/menus/save.asm";
const timePath = "home/time.asm";
const defaultOptionsPath = "data/default_options.asm";
const homeMenuPath = "home/menu.asm";
const copyPath = "home/copy.asm";
const tilemapPath = "home/tilemap.asm";
const palettesPath = "home/palettes.asm";
const textPath = "home/text.asm";
const charmapPath = "constants/charmap.asm";
const coordsMacrosPath = "macros/coords.asm";
const vramPath = "ram/vram.asm";
const gbcOnlyPath = "engine/movie/gbc_only.asm";
const joypadPath = "home/joypad.asm";
const miscGraphicsPath = "gfx/misc.asm";
const loadFontPath = "engine/gfx/load_font.asm";
const cgbLayoutsPath = "engine/gfx/cgb_layouts.asm";
const sgbLayoutsPath = "engine/gfx/sgb_layouts.asm";
const predefPalettesPath = "gfx/sgb/predef.pal";
const runtimePresentationAudioIds = new Set([
  "MUSIC_NONE",
  "MUSIC_TITLE",
  "MUSIC_MAIN_MENU",
  "MUSIC_CRYSTAL_OPENING",
  "MUSIC_ROUTE_30",
  "MUSIC_MOBILE_ADAPTER_MENU",
  "SFX_TITLE_SCREEN_ENTRANCE",
  "SFX_GAME_FREAK_PRESENTS",
  "SFX_DITTO_BOUNCE",
  "SFX_DITTO_POP_UP",
  "SFX_DITTO_TRANSFORM",
  "SFX_INTRO_UNOWN_1",
  "SFX_INTRO_UNOWN_2",
  "SFX_INTRO_UNOWN_3",
  "SFX_INTRO_SUICUNE_2",
  "SFX_INTRO_SUICUNE_3",
  "SFX_INTRO_SUICUNE_4",
  "SFX_INTRO_PICHU",
  "SFX_INTRO_WHOOSH",
  "SFX_ESCAPE_ROPE",
  "CRY_WOOPER",
]);

const canonicalRead = (relativePath: string): string =>
  fs.readFileSync(path.join(disassemblyRoot, relativePath), "utf8");

const replaceExact = (
  source: string,
  before: string,
  after: string,
): string => {
  expect(source.split(before)).toHaveLength(2);
  return source.replace(before, after);
};

const replaceFirstExact = (
  source: string,
  before: string,
  after: string,
): string => {
  const index = source.indexOf(before);
  expect(index).toBeGreaterThanOrEqual(0);
  return `${source.slice(0, index)}${after}${source.slice(index + before.length)}`;
};

type SpriteControlFlowView = {
  indirect_tables: Array<{
    table: string;
    index_domain: { values: number[] } | null;
  }>;
  sprite_operations: Array<Record<string, unknown>>;
  sprite_programs: Array<Record<string, any>>;
  sprite_diagnostics: Array<{
    table: string;
    message: string;
    source_span: { file: string; start_line: number; end_line: number };
  }>;
};

const spriteView = (value: unknown): SpriteControlFlowView =>
  value as SpriteControlFlowView;

const crystalIntroOperations = (
  checkpoint: ReturnType<typeof analyzeRuntimeTitlePresentationEmission>,
): Array<Record<string, any>> =>
  checkpoint.subprograms
    .find((subprogram) => subprogram.id === "crystal_intro")
    ?.phases.find((phase) => phase.id === "scene_dispatch")?.operations ?? [];

describe("runtime title presentation source CFG", () => {
  it("emits the exact TryLoadSaveData host effect before the next unsupported effect", () => {
    const checkpoint = analyzeRuntimeTitlePresentationEmission({
      disassemblyRoot,
      audioAssetIds: new Set(),
      runtimeSpawnIdentifiers: new Set([0]),
    });
    expect(checkpoint.entrypoints).toEqual({
      boot: "GameInit",
      intro: "IntroSequence",
      title: "StartTitleScreen",
      main_menu: "Intro_MainMenu",
      continue: "Continue",
      new_game: "NewGame",
      delete_save: "DeleteSaveData",
      reset_clock: "ResetClock",
    });
    expect(checkpoint.blocks).toEqual({
      ".Check1Pass@Continue": {
        source_span: { file: introPath, start_line: 353, end_line: 357 },
        operations: [
          {
            op: "call_subprogram",
            program: "continue_rtc_restart_clock",
            result: "continue_rtc_result",
            source_span: { file: introPath, start_line: 354, end_line: 354 },
          },
          {
            op: "branch_result",
            result: "continue_rtc_result",
            equals: "passed",
            target: ".Check2Pass@Continue",
            source_span: { file: introPath, start_line: 355, end_line: 355 },
          },
          {
            op: "close_window",
            preserve: ["a", "flags"],
            pop_window_stack: true,
            restore_tile_backup: true,
            apply_tilemap: true,
            update_sprites: true,
            invocation_source_span: {
              file: introPath,
              start_line: 356,
              end_line: 356,
            },
            source_span: expect.any(Object),
            implementation_source_spans: expect.any(Array),
          },
          {
            op: "jump",
            target: ".FailToLoad@Continue",
            source_span: { file: introPath, start_line: 357, end_line: 357 },
          },
        ],
      },
      ".Check2Pass@Continue": {
        source_span: { file: introPath, start_line: 359, end_line: 380 },
        operations: expect.arrayContaining([
          expect.objectContaining({
            op: "request_music_fade",
            frames: 8,
            audio: "MUSIC_NONE",
          }),
          expect.objectContaining({
            op: "skip_unreachable_mobile_adapter_menu",
            reason: "English CheckMobileAdapterStatus clears carry",
          }),
          expect.objectContaining({ op: "jump_roaming_mons" }),
          expect.objectContaining({ op: "copy_received_mystery_gift_decorations" }),
          expect.objectContaining({ op: "continue_clock" }),
          expect.objectContaining({
            op: "branch_spawn_after_champion",
            champion_spawn: "SPAWN_LANCE",
            champion_destination: "SPAWN_NEW_BARK",
            ordinary_map_entry: "MAPSETUP_CONTINUE",
            champion_map_entry: "MAPSETUP_WARP",
          }),
        ]),
      },
      ".FailToLoad@Continue": {
        source_span: { file: introPath, start_line: 382, end_line: 383 },
        operations: [
          {
            op: "return",
            source_span: { file: introPath, start_line: 383, end_line: 383 },
          },
        ],
      },
      ".loop@FinishContinueFunction": {
        source_span: { file: introPath, start_line: 460, end_line: 473 },
        operations: [
          expect.objectContaining({
            op: "prepare_overworld_session",
            clears: ["wDontPlayMapMusicOnReload", "wLinkMode"],
            game_timer_counting: true,
            game_timer_mobile: false,
            show_map_name_sign: true,
          }),
          expect.objectContaining({
            op: "run_overworld_loop",
            status: "wMapStatus",
            initial_status: "MAPSTATUS_START",
            terminal_status: "MAPSTATUS_DONE",
            dispatch: ["StartMap", "EnterMap", "HandleMap", ".done@OverworldLoop"],
          }),
          expect.objectContaining({
            op: "handle_overworld_return",
            red_spawn: "SPAWN_RED",
            red_destination: "SPAWN_MT_SILVER",
            red_map_entry: "MAPSETUP_WARP",
            ordinary_destination: "Reset",
          }),
        ],
      },
      FinishContinueFunction: {
        source_span: { file: introPath, start_line: 459, end_line: 459 },
        operations: [
          {
            op: "jump",
            target: ".loop@FinishContinueFunction",
            source_span: { file: introPath, start_line: 459, end_line: 459 },
          },
        ],
      },
      Init: {
        source_span: { file: "home/init.asm", start_line: 35, end_line: 56 },
        operations: [
          expect.objectContaining({
            op: "initialize_machine",
            interrupts: { disable_during_init: true, final_enable_mask: "IE_DEFAULT" },
            timer_frequency_hz: 4096,
            lcd_disable_scanline: "LY_VBLANK + 1",
            cleared_memory: [
              { region: "WRAM0", banks: [0] },
              { region: "HRAM", preserved: ["hCGB", "hSystemBooted"] },
              { region: "WRAMX", banks: [1], source_bug: "jr nc, .bank_loop" },
              { region: "VRAM", banks: [1, 0] },
              { region: "sScratch", byte_count: 32 },
            ],
            final_lcd_control: "LCDC_DEFAULT",
            final_window: { x: "WX_OFS", y: "SCREEN_HEIGHT_PX" },
          }),
          {
            op: "jump",
            target: "GameInit",
            source_span: { file: "home/init.asm", start_line: 168, end_line: 168 },
          },
        ],
      },
      GameInit: {
        source_span: {
          file: introPath,
          start_line: 1329,
          end_line: 1344,
        },
        operations: [
          {
            op: "host_effect",
            effect: "try_load_save_data",
            result: "save_source",
            args: [],
            source_span: {
              file: introPath,
              start_line: 1330,
              end_line: 1330,
            },
          },
          {
            op: "fill_memory",
            target: "wMenuMetadata",
            byte_count: 16,
            value: 0,
            direction: "ascending",
            bank: { select: "current", restore: false },
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: homeMenuPath,
              start_line: 757,
              end_line: 758,
            },
          },
          {
            op: "fill_memory",
            target: "wMenuHeader",
            byte_count: 16,
            value: 0,
            direction: "ascending",
            bank: { select: "current", restore: false },
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: homeMenuPath,
              start_line: 759,
              end_line: 760,
            },
          },
          {
            op: "fill_memory",
            target: "wMenuData",
            byte_count: 16,
            value: 0,
            direction: "ascending",
            bank: { select: "current", restore: false },
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: homeMenuPath,
              start_line: 761,
              end_line: 762,
            },
          },
          {
            op: "fill_memory",
            target: "wMoreMenuData",
            byte_count: 16,
            value: 0,
            direction: "ascending",
            bank: { select: "current", restore: false },
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: homeMenuPath,
              start_line: 763,
              end_line: 764,
            },
          },
          {
            op: "fill_memory",
            target: "wWindowStackBottom",
            byte_count: 2,
            value: 0,
            direction: "descending",
            bank: { select: "BANK(wWindowStack)", restore: true },
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: homeMenuPath,
              start_line: 766,
              end_line: 781,
            },
          },
          {
            op: "write_memory_word",
            target: "wWindowStackPointer",
            value: "wWindowStackBottom - 2",
            byte_order: "little_endian",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: homeMenuPath,
              start_line: 775,
              end_line: 778,
            },
          },
          {
            op: "write_memory_byte",
            target: "rBGP",
            value: 0,
            address_space: "hardware_register",
            condition: {
              source: "hCGB",
              predicate: "zero",
              source_span: {
                file: tilemapPath,
                start_line: 172,
                end_line: 174,
              },
            },
            source_span: {
              file: tilemapPath,
              start_line: 177,
              end_line: 178,
            },
          },
          {
            op: "write_memory_byte",
            target: "rOBP0",
            value: 0,
            address_space: "hardware_register",
            condition: {
              source: "hCGB",
              predicate: "zero",
              source_span: {
                file: tilemapPath,
                start_line: 172,
                end_line: 174,
              },
            },
            source_span: {
              file: tilemapPath,
              start_line: 177,
              end_line: 179,
            },
          },
          {
            op: "write_memory_byte",
            target: "rOBP1",
            value: 0,
            address_space: "hardware_register",
            condition: {
              source: "hCGB",
              predicate: "zero",
              source_span: {
                file: tilemapPath,
                start_line: 172,
                end_line: 174,
              },
            },
            source_span: {
              file: tilemapPath,
              start_line: 177,
              end_line: 180,
            },
          },
          {
            op: "fill_memory",
            target: "wBGPals2",
            byte_count: 128,
            value: 0xff,
            direction: "ascending",
            bank: { select: "BANK(wBGPals2)", restore: true },
            condition: {
              source: "hCGB",
              predicate: "nonzero",
              source_span: {
                file: tilemapPath,
                start_line: 172,
                end_line: 174,
              },
            },
            source_span: {
              file: tilemapPath,
              start_line: 184,
              end_line: 197,
            },
          },
          {
            op: "palette_transfer_request",
            condition: {
              source: "hCGB",
              predicate: "nonzero",
              source_span: {
                file: tilemapPath,
                start_line: 172,
                end_line: 174,
              },
            },
            request: {
              target: "hCGBPalUpdate",
              queued_value: 1,
              completion_value: 0,
            },
            background: {
              source: "wBGPals2",
              byte_count: 64,
              target: "cgb_background_palette_ram",
              index_register: "rBGPI",
              data_register: "rBGPD",
              autoincrement: true,
            },
            objects: {
              source: "wOBPals2",
              byte_count: 64,
              target: "cgb_object_palette_ram",
              index_register: "rOBPI",
              data_register: "rOBPD",
              autoincrement: true,
            },
            schedule: "vblank",
            source_span: {
              file: tilemapPath,
              start_line: 200,
              end_line: 201,
            },
            implementation_source_spans: [
              {
                file: palettesPath,
                start_line: 3,
                end_line: 67,
              },
              {
                file: "home/vblank.asm",
                start_line: 96,
                end_line: 102,
              },
            ],
          },
          {
            op: "write_memory_byte",
            target: "hBGMapMode",
            value: 1,
            address_space: "hram",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: tilemapPath,
              start_line: 5,
              end_line: 6,
            },
          },
          {
            op: "wait_frames",
            frames: 4,
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: tilemapPath,
              start_line: 8,
              end_line: 9,
            },
          },
          {
            op: "fill_memory",
            target: "wTilemap",
            byte_count: 360,
            value: 0x7f,
            direction: "ascending",
            bank: { select: "current", restore: false },
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            value_source_span: {
              file: charmapPath,
              start_line: 73,
              end_line: 73,
            },
            source_span: {
              file: textPath,
              start_line: 25,
              end_line: 28,
            },
          },
          {
            op: "write_memory_byte",
            target: "hBGMapMode",
            value: 1,
            address_space: "hram",
            condition: {
              source: "rLCDC",
              predicate: "bit_set",
              bit: { symbol: "B_LCDC_ENABLE", value: 7 },
              source_span: {
                file: textPath,
                start_line: 31,
                end_line: 33,
              },
            },
            source_span: {
              file: tilemapPath,
              start_line: 5,
              end_line: 6,
            },
          },
          {
            op: "wait_frames",
            frames: 4,
            condition: {
              source: "rLCDC",
              predicate: "bit_set",
              bit: { symbol: "B_LCDC_ENABLE", value: 7 },
              source_span: {
                file: textPath,
                start_line: 31,
                end_line: 33,
              },
            },
            source_span: {
              file: tilemapPath,
              start_line: 8,
              end_line: 9,
            },
          },
          {
            op: "write_memory_byte",
            target: "hBGMapAddress + 1",
            value: "HIGH(vBGMap0)",
            address_space: "hram",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            value_source_span: {
              file: vramPath,
              start_line: 6,
              end_line: 6,
            },
            source_span: {
              file: introPath,
              start_line: 1334,
              end_line: 1335,
            },
          },
          {
            op: "write_memory_byte",
            target: "hBGMapAddress",
            value: 0,
            address_space: "hram",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            value_source_span: {
              file: introPath,
              start_line: 1336,
              end_line: 1336,
            },
            source_span: {
              file: introPath,
              start_line: 1336,
              end_line: 1337,
            },
          },
          {
            op: "write_memory_byte",
            target: "hJoyDown",
            value: 0,
            address_space: "hram",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            value_source_span: {
              file: introPath,
              start_line: 1336,
              end_line: 1336,
            },
            source_span: {
              file: introPath,
              start_line: 1336,
              end_line: 1338,
            },
          },
          {
            op: "write_memory_byte",
            target: "hSCX",
            value: 0,
            address_space: "hram",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            value_source_span: {
              file: introPath,
              start_line: 1336,
              end_line: 1336,
            },
            source_span: {
              file: introPath,
              start_line: 1336,
              end_line: 1339,
            },
          },
          {
            op: "write_memory_byte",
            target: "hSCY",
            value: 0,
            address_space: "hram",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            value_source_span: {
              file: introPath,
              start_line: 1336,
              end_line: 1336,
            },
            source_span: {
              file: introPath,
              start_line: 1336,
              end_line: 1340,
            },
          },
          {
            op: "write_memory_byte",
            target: "hWY",
            value: 0x90,
            address_space: "hram",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            value_source_span: {
              file: introPath,
              start_line: 1341,
              end_line: 1341,
            },
            source_span: {
              file: introPath,
              start_line: 1341,
              end_line: 1342,
            },
          },
          {
            op: "write_memory_byte",
            target: "hBGMapMode",
            value: 1,
            address_space: "hram",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: tilemapPath,
              start_line: 5,
              end_line: 6,
            },
          },
          {
            op: "wait_frames",
            frames: 4,
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: tilemapPath,
              start_line: 8,
              end_line: 9,
            },
          },
          {
            op: "jump",
            target: "IntroSequence",
            source_span: {
              file: introPath,
              start_line: 1344,
              end_line: 1344,
            },
          },
        ],
      },
      IntroSequence: {
        source_span: {
          file: introPath,
          start_line: 964,
          end_line: 967,
        },
        operations: [
          {
            op: "call_subprogram",
            program: "splash_screen",
            result: "splash_outcome",
            source_span: {
              file: introPath,
              start_line: 965,
              end_line: 965,
            },
          },
          {
            op: "branch_result",
            result: "splash_outcome",
            equals: "cancelled",
            target: "StartTitleScreen",
            source_span: {
              file: introPath,
              start_line: 966,
              end_line: 966,
            },
          },
          {
            op: "call_subprogram",
            program: "crystal_intro",
            result: "crystal_intro_preserved_carry",
            source_span: {
              file: introPath,
              start_line: 967,
              end_line: 967,
            },
          },
        ],
      },
      StartTitleScreen: {
        source_span: {
          file: introPath,
          start_line: 971,
          end_line: 978,
        },
        operations: [
          {
            op: "save_memory_byte",
            source: "rWBK",
            storage: { kind: "cpu_stack", register_pair: "af" },
            restore_required: true,
            source_span: { file: introPath, start_line: 972, end_line: 973 },
          },
          {
            op: "write_memory_byte",
            target: "rWBK",
            address_space: "hardware_register",
            value: "BANK(wLYOverrides)",
            value_source_span: {
              file: wramPath,
              start_line: 3517,
              end_line: 3517,
            },
            source_span: { file: introPath, start_line: 974, end_line: 975 },
          },
          {
            op: "call_subprogram",
            program: "start_title_screen",
            result: "title_screen_tail_destination",
            source_span: { file: introPath, start_line: 977, end_line: 977 },
          },
        ],
      },
      Intro_MainMenu: {
        source_span: { file: introPath, start_line: 1, end_line: 10 },
        operations: [
          {
            op: "stop_audio",
            audio: "MUSIC_NONE",
            source_span: { file: introPath, start_line: 2, end_line: 3 },
          },
          {
            op: "wait_frames",
            frames: 1,
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: { file: introPath, start_line: 4, end_line: 4 },
          },
          {
            op: "write_memory_byte",
            target: "wMapMusic",
            address_space: "wram",
            value: "MUSIC_MAIN_MENU",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: { file: introPath, start_line: 5, end_line: 7 },
          },
          {
            op: "play_audio",
            audio: "MUSIC_MAIN_MENU",
            source_span: { file: introPath, start_line: 5, end_line: 8 },
          },
          {
            op: "call_subprogram",
            program: "main_menu",
            result: "main_menu_outcome",
            source_span: { file: introPath, start_line: 9, end_line: 9 },
          },
          {
            op: "jump",
            target: "StartTitleScreen",
            source_span: { file: introPath, start_line: 10, end_line: 10 },
          },
        ],
      },
      Continue: {
        source_span: { file: introPath, start_line: 338, end_line: 351 },
        operations: expect.arrayContaining([
          {
            op: "call_subprogram",
            program: "try_load_save_file",
            result: "save_file_load_outcome",
            source_span: { file: introPath, start_line: 339, end_line: 339 },
          },
          {
            op: "branch_result",
            result: "save_file_load_outcome",
            equals: "corrupt",
            target: ".FailToLoad@Continue",
            source_span: { file: introPath, start_line: 340, end_line: 340 },
          },
          expect.objectContaining({
            op: "copy_memory",
            source: "sCrystalData",
            target: "wCrystalData",
            byte_count: "wCrystalDataEnd - wCrystalData",
            source_bank: "BANK(sCrystalData)",
            restore_sram: true,
          }),
          expect.objectContaining({
            op: "copy_memory",
            source: "sCrystalFlags",
            target: "wCrystalFlags",
            byte_count: 2,
            source_bank: "BANK(sCrystalData)",
            restore_sram: true,
          }),
          expect.objectContaining({
            op: "load_menu_header",
            header: ".MenuHeader@LoadStandardMenuHeader",
            flags: ["MENU_BACKUP_TILES"],
            coordinates: { left: 0, top: 0, right: 19, bottom: 17 },
            default_option: 1,
            pushes_window: true,
          }),
        ]),
      },
      NewGame: {
        source_span: { file: introPath, start_line: 61, end_line: 78 },
        operations: [
          {
            op: "write_memory_byte",
            target: "wDebugFlags",
            address_space: "wram",
            value: 0,
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: { file: introPath, start_line: 62, end_line: 63 },
          },
          {
            op: "call_subprogram",
            program: "reset_wram",
            result: "reset_wram_completion",
            source_span: { file: introPath, start_line: 64, end_line: 64 },
          },
          {
            op: "call_subprogram",
            program: "new_game_clear_tilemap",
            result: "new_game_display_preparation_completion",
            source_span: { file: introPath, start_line: 65, end_line: 65 },
          },
          {
            op: "call_subprogram",
            program: "player_profile_setup",
            result: "player_profile_setup_completion",
            source_span: { file: introPath, start_line: 66, end_line: 66 },
          },
          {
            op: "call_subprogram",
            program: "oak_speech",
            result: "oak_speech_completion",
            source_span: { file: introPath, start_line: 67, end_line: 67 },
          },
          {
            op: "call_subprogram",
            program: "initialize_world",
            result: "initialize_world_completion",
            source_span: { file: introPath, start_line: 68, end_line: 68 },
          },
          {
            op: "write_memory_byte",
            target: "wPrevLandmark",
            address_space: "wram",
            value: "LANDMARK_NEW_BARK_TOWN",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: { file: introPath, start_line: 70, end_line: 71 },
          },
          {
            op: "write_memory_byte",
            target: "wDefaultSpawnpoint",
            address_space: "wram",
            value: "SPAWN_HOME",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: { file: introPath, start_line: 73, end_line: 74 },
          },
          {
            op: "write_memory_byte",
            target: "hMapEntryMethod",
            address_space: "hram",
            value: "MAPSETUP_WARP",
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: { file: introPath, start_line: 76, end_line: 77 },
          },
          {
            op: "jump",
            target: "FinishContinueFunction",
            source_span: { file: introPath, start_line: 78, end_line: 78 },
          },
        ],
      },
      DeleteSaveData: {
        source_span: { file: introPath, start_line: 1256, end_line: 1258 },
        operations: [
          {
            op: "call_subprogram",
            program: "delete_save_data",
            result: "delete_save_completion",
            source_span: { file: introPath, start_line: 1257, end_line: 1257 },
          },
          {
            op: "jump",
            target: "Init",
            source_span: { file: introPath, start_line: 1258, end_line: 1258 },
          },
        ],
      },
      ResetClock: {
        source_span: { file: introPath, start_line: 1260, end_line: 1262 },
        operations: [
          {
            op: "call_subprogram",
            program: "reset_clock_password",
            result: "reset_clock_completion",
            source_span: { file: introPath, start_line: 1261, end_line: 1261 },
          },
          {
            op: "jump",
            target: "Init",
            source_span: { file: introPath, start_line: 1262, end_line: 1262 },
          },
        ],
      },
    });
    expect(checkpoint.host_effects).toEqual([
      {
        id: "try_load_save_data",
        call_target: "TryLoadSaveData",
        accepted_call_forms: ["farcall"],
        result: {
          name: "save_source",
          type: "enum",
          domain: [
            {
              id: "none",
              value: 0,
              conditions: [
                { source: "primary", valid: false },
                { source: "backup", valid: false },
              ],
            },
            {
              id: "primary",
              value: 1,
              conditions: [{ source: "primary", valid: true }],
            },
            {
              id: "backup",
              value: 2,
              conditions: [
                { source: "primary", valid: false },
                { source: "backup", valid: true },
              ],
            },
          ],
        },
        validity_checks: [
          {
            source: "primary",
            fields: [
              { name: "sCheckValue1", equals: 99 },
              { name: "sCheckValue2", equals: 127 },
            ],
            source_span: { file: savePath, start_line: 692, end_line: 711 },
          },
          {
            source: "backup",
            fields: [
              { name: "sBackupCheckValue1", equals: 99 },
              { name: "sBackupCheckValue2", equals: 127 },
            ],
            source_span: { file: savePath, start_line: 713, end_line: 731 },
          },
        ],
        state_deltas: [
          expect.objectContaining({
            when: "primary",
            writes: expect.arrayContaining([
              expect.objectContaining({
                kind: "copy_bytes",
                source: "sOptions",
                target: "wOptions",
                byte_count: 8,
                fields: [
                  "wOptions",
                  "wSaveFileExists",
                  "wTextboxFrame",
                  "wTextboxFlags",
                  "wGBPrinterBrightness",
                  "wOptions2",
                  "wOptions + 6",
                  "wOptions + 7",
                ],
              }),
              expect.objectContaining({
                kind: "constant_byte",
                target: "wSaveFileExists",
                value: 1,
              }),
              expect.objectContaining({
                kind: "copy_bytes",
                source: "sPlayerData + wStartDay - wPlayerData",
                target: "wStartDay",
                byte_count: 8,
                fields: [
                  "wStartDay",
                  "wStartHour",
                  "wStartMinute",
                  "wStartSecond",
                  "wRTC + 0",
                  "wRTC + 1",
                  "wRTC + 2",
                  "wRTC + 3",
                ],
              }),
              expect.objectContaining({
                kind: "copy_byte",
                source: "sPlayerData + wStatusFlags - wPlayerData",
                target: "wStatusFlags",
              }),
            ]),
          }),
          expect.objectContaining({
            when: "backup",
            writes: expect.arrayContaining([
              expect.objectContaining({
                kind: "copy_bytes",
                source: "sBackupOptions",
                target: "wOptions",
                byte_count: 8,
              }),
              expect.objectContaining({
                kind: "constant_byte",
                target: "wSaveFileExists",
                value: 2,
              }),
              expect.objectContaining({
                kind: "copy_bytes",
                source: "sBackupPlayerData + wStartDay - wPlayerData",
                target: "wStartDay",
                byte_count: 8,
              }),
              expect.objectContaining({
                kind: "copy_byte",
                source: "sBackupPlayerData + wStatusFlags - wPlayerData",
                target: "wStatusFlags",
              }),
            ]),
          }),
          expect.objectContaining({
            when: "none",
            writes: expect.arrayContaining([
              expect.objectContaining({
                kind: "copy_bytes",
                source: "DefaultOptions",
                target: "wOptions",
                byte_count: 8,
                values: [
                  "TEXT_DELAY_MED",
                  "FALSE",
                  "FRAME_1",
                  "1 << FAST_TEXT_DELAY_F",
                  "GBPRINTER_NORMAL",
                  "1 << MENU_ACCOUNT",
                  "$00",
                  "$00",
                ],
              }),
              expect.objectContaining({
                kind: "zero_bytes",
                targets: [
                  "hRTCSeconds",
                  "hRTCMinutes",
                  "hRTCHours",
                  "hRTCDayLo",
                  "hRTCDayHi",
                ],
              }),
              expect.objectContaining({
                kind: "persist_rtc",
                sources: [
                  "hRTCSeconds",
                  "hRTCMinutes",
                  "hRTCHours",
                  "hRTCDayLo",
                  "hRTCDayHi",
                ],
                targets: [
                  "RAMB_RTC_S",
                  "RAMB_RTC_M",
                  "RAMB_RTC_H",
                  "RAMB_RTC_DL",
                  "RAMB_RTC_DH",
                ],
                clears_halt: true,
              }),
            ]),
          }),
        ],
        required_consumer: {
          id: "runtime_title_screen.try_load_save_data",
          required: true,
        },
        source_span: { file: savePath, start_line: 642, end_line: 688 },
        implementation_source_spans: [
          { file: savePath, start_line: 642, end_line: 688 },
          { file: savePath, start_line: 692, end_line: 711 },
          { file: savePath, start_line: 713, end_line: 731 },
          { file: defaultOptionsPath, start_line: 1, end_line: 18 },
          { file: wramPath, start_line: 1782, end_line: 1811 },
          { file: wramPath, start_line: 2895, end_line: 2900 },
          { file: wramPath, start_line: 2960, end_line: 2969 },
          { file: "ram/sram.asm", start_line: 60, end_line: 103 },
          {
            file: "constants/misc_constants.asm",
            start_line: 26,
            end_line: 27,
          },
          { file: timePath, start_line: 198, end_line: 257 },
        ],
      },
    ]);
    const startTitleScreen = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "start_title_screen",
    );
    const titleScreenOperations =
      startTitleScreen?.phases.find((phase) => phase.id === "title_screen")
        ?.operations ?? [];
    expect(checkpoint.frontier).toBeNull();
    expect({
      ...checkpoint.frontier,
      compiled_prefix: {
        source_entry: ".TitleScreen@StartTitleScreen",
        block: "_TitleScreen",
        operations: titleScreenOperations,
      },
    }).toMatchObject({
      compiled_prefix: {
        source_entry: ".TitleScreen@StartTitleScreen",
        block: "_TitleScreen",
        operations: expect.arrayContaining([
          expect.objectContaining({
            op: "source_wrapper",
            call_form: "farcall",
            target: "_TitleScreen",
            returns: true,
            source_span: {
              file: introPath,
              start_line: 1027,
              end_line: 1028,
            },
          }),
          expect.objectContaining({
            op: "fill_memory",
            target: "wJumptableIndex",
            byte_count: 4,
            value: 0,
            destination_labels: [
              "wJumptableIndex",
              "wTitleScreenSelectedOption",
              "wTitleScreenTimer",
            ],
          }),
          expect.objectContaining({
            op: "disable_lcd",
            wait_until: { source: "rLY", equals: 145 },
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource_symbol: "TitleSuicuneGFX",
            target: "vTiles4",
            target_vram_bank: 1,
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource_symbol: "TitleLogoGFX",
            target: "vTiles1",
            target_vram_bank: 0,
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource_symbol: "TitleCrystalGFX",
            target: "vTiles0",
            target_vram_bank: 0,
          }),
          expect.objectContaining({
            op: "initialize_title_crystal_oam",
            columns: 5,
            objects_per_column: 6,
          }),
          expect.objectContaining({
            op: "write_memory_pattern",
            target: "wLYOverrides",
            byte_count: 80,
            pattern: [112, 144],
            repeat_count: 40,
          }),
          expect.objectContaining({
            op: "play_audio",
            audio: "SFX_TITLE_SCREEN_ENTRANCE",
          }),
          expect.objectContaining({
            op: "dispatch_table",
            dispatcher: "TitleScreenScene",
            entries: [
              "TitleScreenEntrance",
              "TitleScreenTimer",
              "TitleScreenMain",
              "TitleScreenEnd",
            ],
            domain: { minimum: 0, maximum: 3, values: [0, 1, 2, 3] },
          }),
          expect.objectContaining({
            op: "draw_indexed_title_suicune_frame",
            frames: [0x80, 0x88, 0x00, 0x08],
            bg_map_mode: { before: 0, after: 1, third: 3 },
          }),
          expect.objectContaining({
            op: "animate_title_crystal",
            stop_at: 22,
            object_count: 30,
            y_delta: 2,
          }),
          expect.objectContaining({
            op: "write_memory_word",
            target: "wTitleScreenTimer",
            value: 4416,
            byte_order: "little_endian",
          }),
          expect.objectContaining({
            op: "decrement_memory_word_unless_zero",
            target: "wTitleScreenTimer",
            zero_target: ".end@TitleScreenMain",
          }),
          expect.objectContaining({
            op: "input_chord_branch",
            sample: "hJoyDown",
            mask: 0x46,
            target: ".delete_save_data@TitleScreenMain",
          }),
          expect.objectContaining({
            op: "input_chord_branch",
            sample: "hJoyDown",
            mask: 0x86,
            target: ".check_start@TitleScreenMain",
          }),
          expect.objectContaining({
            op: "fade_audio",
            audio: "MUSIC_NONE",
            frames: 64,
            fade_register: { target: "wMusicFade", value: 8 },
          }),
          expect.objectContaining({
            op: "return_if_memory_nonzero",
            source: "wMusicFade",
          }),
          expect.objectContaining({
            op: "clear_memory_bit",
            target: "rLCDC",
            bit: 2,
          }),
          expect.objectContaining({
            op: "fill_memory",
            target: "wAttrmap",
            byte_count: 360,
            value: 7,
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hBGMapMode",
            value: 2,
            condition: { source: "hCGB", predicate: "nonzero" },
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hWY",
            value: 0x90,
          }),
        ]),
      },
    });
    expect(startTitleScreen).toMatchObject({
      source_entry: ".TitleScreen",
      accepted_call_forms: ["call"],
      result: {
        name: "title_screen_tail_destination",
        domain: [
          expect.objectContaining({
            value: null,
            condition: expect.objectContaining({
              value: 0,
              destination: "Intro_MainMenu",
            }),
          }),
          expect.objectContaining({
            value: null,
            condition: expect.objectContaining({
              value: 1,
              destination: "DeleteSaveData",
            }),
          }),
          expect.objectContaining({
            value: null,
            condition: expect.objectContaining({
              value: 2,
              destination: "IntroSequence",
            }),
          }),
          expect.objectContaining({
            value: null,
            condition: expect.objectContaining({
              value: 3,
              destination: "IntroSequence",
            }),
          }),
          expect.objectContaining({
            value: null,
            condition: expect.objectContaining({
              value: 4,
              destination: "ResetClock",
            }),
          }),
        ],
      },
      loop: { scheduler: null, natural_scheduler_ticks: null },
      required_consumer: {
        id: "runtime_title_screen.start_title_screen",
        required: true,
      },
    });
    expect(titleScreenOperations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "apply_palette_layout",
          layout: { symbol: "SCGB_DIPLOMA", value: 8 },
        }),
        expect.objectContaining({
          op: "update_time_palettes",
          fade_index: 9,
          source: "GetTimePalFade",
          converter: "DmgToCgbTimePals",
        }),
        expect.objectContaining({
          op: "clamp_memory_byte",
          source: "wTitleScreenSelectedOption",
          valid_max_exclusive: 5,
          replacement: 0,
        }),
        expect.objectContaining({
          op: "dispatch_table",
          dispatcher: "StartTitleScreen option tail",
          entries: [
            "Intro_MainMenu",
            "DeleteSaveData",
            "IntroSequence",
            "IntroSequence",
            "ResetClock",
          ],
          tail_dispatch: true,
        }),
      ]),
    );
    const mainMenu = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "main_menu",
    );
    expect(mainMenu).toMatchObject({
      source_entry: "MainMenu",
      accepted_call_forms: ["farcall"],
      result: {
        name: "main_menu_outcome",
        domain: expect.arrayContaining([
          expect.objectContaining({ id: "cancelled", value: 1 }),
          expect.objectContaining({ id: "new_game_non_returning", value: null }),
          expect.objectContaining({
            id: "continue_success_non_returning",
            value: null,
          }),
        ]),
      },
      loop: {
        scene_dispatch: {
          table: ".Jumptable@MainMenu",
          index: "wMenuSelection",
          domain: { minimum: 0, maximum: 3, values: [0, 1, 2, 3] },
        },
        scheduler: null,
      },
      required_consumer: {
        id: "runtime_title_screen.main_menu",
        required: true,
      },
    });
    const mainMenuOperations =
      mainMenu?.phases.find((phase) => phase.id === "main_menu")?.operations ?? [];
    expect(
      mainMenu?.phases.find((phase) => phase.id === "main_menu")?.labels,
    ).toEqual({
      ".loop@MainMenu": 0,
      ".quit@MainMenu": 14,
    });
    expect(mainMenuOperations.slice(-2)).toEqual([
      {
        op: "jump",
        target: ".loop@MainMenu",
        source_span: { file: mainMenuPath, start_line: 48, end_line: 48 },
      },
      {
        op: "return",
        source_span: { file: mainMenuPath, start_line: 50, end_line: 51 },
      },
    ]);
    expect(mainMenuOperations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "select_main_menu_variant",
          result: "wWhichIndexSet",
          variants: [
            expect.objectContaining({ value: 0, id: "new_game" }),
            expect.objectContaining({ value: 1, id: "continue" }),
            expect.objectContaining({ value: 6, id: "mystery" }),
          ],
        }),
        expect.objectContaining({
          op: "load_menu",
          item_records: "MainMenuItems",
          coordinates: { left: 0, top: 0, right: 16, bottom: 7 },
          default_option: 1,
          item_sets: [
            [1, 2],
            [0, 1, 2],
            [0, 1, 2, 3, 4],
            [0, 1, 2, 4],
            [0, 1, 2, 4, 5],
            [0, 1, 2, 3, 4, 5],
            [0, 1, 2, 3],
            [0, 1, 2, 3, 5],
            [0, 1, 2, 5],
          ],
          strings: [
            "CONTINUE",
            "NEW GAME",
            "OPTION",
            "MYSTERY GIFT",
            "MOBILE",
            "MOBILE STUDIUM",
          ],
        }),
        expect.objectContaining({
          op: "menu_input_loop",
          sampler: "GetScrollingMenuJoypad",
          accept: "PAD_A",
          cancel: "PAD_B",
          wrap_vertical: true,
        }),
        expect.objectContaining({
          op: "dispatch_table",
          table: ".Jumptable@MainMenu",
          entries: [
            "MainMenu_Continue",
            "MainMenu_NewGame",
            "MainMenu_Option",
            "MainMenu_MysteryGift",
            "MainMenu_Mobile",
            "MainMenu_MobileStudium",
          ],
          domain: { minimum: 0, maximum: 3, values: [0, 1, 2, 3] },
        }),
      ]),
    );
    const tryLoadSaveFile = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "try_load_save_file",
    );
    expect(tryLoadSaveFile).toMatchObject({
      source_entry: "TryLoadSaveFile",
      accepted_call_forms: ["farcall"],
      result: {
        name: "save_file_load_outcome",
        domain: [
          expect.objectContaining({ id: "primary_loaded", value: 0 }),
          expect.objectContaining({ id: "backup_loaded", value: 0 }),
          expect.objectContaining({ id: "corrupt", value: 1 }),
        ],
      },
      required_consumer: {
        id: "runtime_title_screen.try_load_save_file",
        required: true,
      },
    });
    expect(tryLoadSaveFile?.phases[0]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "verify_save_checksum",
          source: "sGameData",
          checksum: "sChecksum",
        }),
        expect.objectContaining({
          op: "load_save_regions",
          source: "primary",
          restores: ["party_mail", "gs_ball_flag", "mystery_gift"],
        }),
        expect.objectContaining({
          op: "verify_save_checksum",
          source: "sBackupGameData",
          checksum: "sBackupChecksum",
        }),
        expect.objectContaining({
          op: "refresh_redundant_save",
          source: "backup",
          target: "primary",
        }),
        expect.objectContaining({
          op: "present_text",
          text: "SaveFileCorruptedText",
          restore_options: true,
        }),
      ]),
    );
    const displaySaveInfo = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "display_save_info_on_continue",
    );
    expect(displaySaveInfo).toMatchObject({
      source_entry: "DisplaySaveInfoOnContinue",
      result: {
        name: "display_save_info_completion",
        storage: "none",
        domain: [expect.objectContaining({ id: "rendered", value: null })],
      },
      required_consumer: {
        id: "runtime_title_screen.display_save_info_on_continue",
        required: true,
      },
    });
    expect(displaySaveInfo?.phases[0]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "select_menu_header",
          bit: "STATUSFLAGS_POKEDEX_F",
          offset: { x: 4, y: 8 },
        }),
        expect.objectContaining({
          op: "render_continue_identity",
          player: { text: "<PLAYER>", coordinate: [8, 2] },
        }),
        expect.objectContaining({
          op: "render_continue_game_time",
          coordinate: [9, 8],
          separator: "<COLON>",
        }),
        expect.objectContaining({
          op: "place_text",
          text: " ???",
          coordinate: [9, 8],
        }),
      ]),
    );
    expect(
      checkpoint.subprograms.find(
        (subprogram) => subprogram.id === "confirm_continue",
      ),
    ).toMatchObject({
      source_entry: "ConfirmContinue",
      result: {
        name: "confirm_continue_outcome",
        domain: [
          expect.objectContaining({ id: "accepted", value: 0 }),
          expect.objectContaining({ id: "cancelled", value: 1 }),
        ],
      },
      required_consumer: {
        id: "runtime_title_screen.confirm_continue",
        required: true,
      },
    });
    const resetWram = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "reset_wram",
    );
    expect(resetWram).toMatchObject({
      source_entry: "ResetWRAM",
      result: {
        name: "reset_wram_completion",
        storage: "none",
        domain: [expect.objectContaining({ id: "completed", value: null })],
      },
      required_consumer: {
        id: "runtime_title_screen.reset_wram",
        required: true,
      },
    });
    expect(resetWram?.phases[0]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "fill_memory_ranges",
          value: 0,
          ranges: [
            { start: "wShadowOAM", byte_count: "wOptions - wShadowOAM" },
            {
              start: "STARTOF(WRAMX)",
              byte_count: "wGameData - STARTOF(WRAMX)",
            },
            { start: "wGameData", byte_count: "wGameDataEnd - wGameData" },
          ],
        }),
        expect.objectContaining({
          op: "initialize_sentinel_lists",
          count_value: 0,
          first_entry_value: 0xff,
        }),
        expect.objectContaining({
          op: "initialize_box_names",
          prefix: "BOX",
          count: 14,
          name_length: 9,
        }),
        expect.objectContaining({
          op: "initialize_economy",
          money: 3000,
          mom_trigger_balance: 2300,
          coins: 0,
        }),
        expect.objectContaining({
          op: "initialize_npc_names",
          values: {
            wRivalName: "???",
            wMomsName: "MOM",
            wRedsName: "RED",
            wGreensName: "GREEN",
          },
        }),
        expect.objectContaining({ op: "clear_saved_mail" }),
        expect.objectContaining({ op: "reset_game_time", value: 0 }),
      ]),
    );
    expect(
      checkpoint.subprograms.find(
        (subprogram) => subprogram.id === "new_game_clear_tilemap",
      ),
    ).toMatchObject({
      source_entry: "NewGame_ClearTilemapEtc",
      result: { storage: "none" },
      required_consumer: {
        id: "runtime_title_screen.new_game_clear_tilemap",
        required: true,
      },
    });
    const playerProfile = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "player_profile_setup",
    );
    expect(playerProfile).toMatchObject({
      source_entry: "PlayerProfileSetup",
      result: { storage: "none" },
      resources: expect.arrayContaining([
        expect.objectContaining({
          path: "gfx/new_game/gender_screen.pal",
          kind: "palette",
        }),
        expect.objectContaining({
          path: "gfx/new_game/gender_screen.2bpp",
          kind: "tiles",
        }),
      ]),
      required_consumer: {
        id: "runtime_title_screen.player_profile_setup",
        required: true,
      },
    });
    expect(playerProfile?.phases[0]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "branch_constant_result",
          source: "CheckMobileAdapterStatus",
          carry: 0,
          selected_path: "InitGender",
          unreachable_path: "InitMobileProfile",
        }),
        expect.objectContaining({
          op: "initialize_crystal_profile",
          prefecture: 1,
          age: 0,
          gender: 0,
          postal_code: [0, 0, 0, 0],
        }),
        expect.objectContaining({
          op: "load_menu",
          items: ["Boy", "Girl"],
          default_option: 1,
        }),
        expect.objectContaining({
          op: "select_player_gender",
          source: "wMenuCursorY - 1",
          target: "wPlayerGender",
        }),
      ]),
    );
    const oakSpeech = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "oak_speech",
    );
    expect(oakSpeech).toMatchObject({
      source_entry: "OakSpeech",
      result: { storage: "none" },
      audio: expect.arrayContaining([
        expect.objectContaining({ id: "MUSIC_ROUTE_30", kind: "music" }),
        expect.objectContaining({ id: "CRY_WOOPER", kind: "cry" }),
      ]),
      required_consumer: {
        id: "runtime_title_screen.oak_speech",
        required: true,
      },
    });
    expect(oakSpeech?.phases[0]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "initialize_clock",
          default_hour: 10,
          hour_range: [0, 23],
          minute_range: [0, 59],
        }),
        expect.objectContaining({
          op: "present_intro_portrait",
          kind: "pokemon",
          species: "WOOPER",
          dvs: [0, 0],
        }),
        expect.objectContaining({
          op: "present_text_sequence",
          entries: ["OakText2", "OakText3", "OakText4"],
        }),
        expect.objectContaining({
          op: "name_player",
          custom_naming_mode: "NAME_PLAYER",
          empty_fallbacks: { male: "CHRIS", female: "KRIS" },
        }),
      ]),
    );
    const initializeWorld = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "initialize_world",
    );
    expect(initializeWorld).toMatchObject({
      source_entry: "InitializeWorld",
      result: { storage: "none" },
      resources: expect.arrayContaining([
        expect.objectContaining({ path: "gfx/new_game/shrink1.2bpp.lz" }),
        expect.objectContaining({ path: "gfx/new_game/shrink2.2bpp.lz" }),
      ]),
      required_consumer: {
        id: "runtime_title_screen.initialize_world",
        required: true,
      },
    });
    expect(initializeWorld?.phases[0]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "animate_player_shrink",
          decompressed_tile_count: 49,
          wait_after_clear: 3,
        }),
        expect.objectContaining({
          op: "place_player_intro_sprite",
          object_count: 4,
          wait_after: 50,
        }),
        expect.objectContaining({
          op: "spawn_player",
          follower_indices: 0xff,
          centered_object: "PLAYER",
        }),
        expect.objectContaining({
          op: "initialize_start_day",
          target: "wTimerEventStartDay",
        }),
      ]),
    );
    const deleteSave = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "delete_save_data",
    );
    expect(deleteSave).toMatchObject({
      source_entry: "_DeleteSaveData",
      accepted_call_forms: ["farcall"],
      result: { storage: "none" },
      required_consumer: {
        id: "runtime_title_screen.delete_save_data",
        required: true,
      },
    });
    expect(deleteSave?.phases[0]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "load_menu",
          items: ["NO", "YES"],
          default_option: 1,
        }),
        expect.objectContaining({
          op: "erase_sram_banks",
          condition: { source: "wMenuCursorY", equals: 2 },
          bank_count: 4,
          start: "STARTOF(SRAM)",
          byte_count: "SIZEOF(SRAM)",
          value: 0,
        }),
      ]),
    );
    const resetClock = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "reset_clock_password",
    );
    expect(resetClock).toMatchObject({
      source_entry: "_ResetClock",
      accepted_call_forms: ["farcall"],
      result: { storage: "none" },
      required_consumer: {
        id: "runtime_title_screen.reset_clock_password",
        required: true,
      },
    });
    expect(resetClock?.phases[0]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "load_menu",
          items: ["NO", "YES"],
          default_option: 1,
        }),
        expect.objectContaining({
          op: "derive_clock_reset_password",
          arithmetic: "wrapping_u16_sum",
          player_id_bytes: 2,
          player_name_bytes: 5,
          money_bytes: 3,
        }),
        expect.objectContaining({
          op: "edit_decimal_password",
          digits: 5,
          initial_digits: [0, 0, 0, 0, 0],
          initial_cursor: 4,
          confirm_button: "PAD_A",
        }),
        expect.objectContaining({
          op: "write_sram_byte",
          target: "sRTCStatusFlags",
          value: { symbol: "RTC_RESET", value: 0x80 },
          condition: { password_matches: true },
        }),
      ]),
    );
    const continueRtc = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "continue_rtc_restart_clock",
    );
    expect(continueRtc).toMatchObject({
      source_entry: "Continue_CheckRTC_RestartClock",
      accepted_call_forms: ["call"],
      result: {
        storage: "carry",
        domain: expect.arrayContaining([
          expect.objectContaining({ id: "passed", value: 0 }),
          expect.objectContaining({ id: "cancelled", value: 1 }),
        ]),
      },
      required_consumer: {
        id: "runtime_title_screen.continue_rtc_restart_clock",
        required: true,
      },
    });
    expect(continueRtc?.phases[0]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "read_sram_byte",
          source: "sRTCStatusFlags",
          mask: { symbol: "RTC_RESET", value: 0x80 },
        }),
        expect.objectContaining({
          op: "edit_restart_clock",
          fields: [
            { id: "day", modulus: 7 },
            { id: "hour", modulus: 24 },
            { id: "minute", modulus: 60 },
          ],
          accept_button: "PAD_A",
          cancel_button: "PAD_B",
        }),
        expect.objectContaining({
          op: "initialize_time_offsets",
          condition: { restart_confirmed: true },
          target_fields: ["wStartDay", "wStartHour", "wStartMinute", "wStartSecond"],
        }),
      ]),
    );
    const crystalIntro = checkpoint.subprograms.find(
      (subprogram) => subprogram.id === "crystal_intro",
    );
    expect(crystalIntro).toBeDefined();
    const crystalIntroScenePhase = crystalIntro?.phases.find(
      (phase) => phase.id === "scene_dispatch",
    );
    expect(crystalIntro).toMatchObject({
      source_entry: "CrystalIntro",
      accepted_call_forms: ["farcall"],
      result: {
        name: "crystal_intro_preserved_carry",
        storage: "carry",
        domain: [
          expect.objectContaining({
            id: "caller_carry_clear",
            value: 0,
            condition: { kind: "preserved_caller_carry", value: 0 },
          }),
          expect.objectContaining({
            id: "caller_carry_set",
            value: 1,
            condition: { kind: "preserved_caller_carry", value: 1 },
          }),
        ],
      },
      phases: expect.arrayContaining([
        expect.objectContaining({
          id: "entry_init",
          operations: expect.arrayContaining([
            expect.objectContaining({ op: "save_memory_byte", source: "rWBK" }),
            expect.objectContaining({ op: "save_memory_byte", source: "hInMenu" }),
            expect.objectContaining({ op: "save_memory_byte", source: "hVBlank" }),
            expect.objectContaining({ op: "write_memory_byte", target: "hVBlank", value: 0 }),
            expect.objectContaining({ op: "write_memory_byte", target: "hInMenu", value: 1 }),
            expect.objectContaining({ op: "write_memory_byte", target: "hMapAnims", value: 0 }),
            expect.objectContaining({ op: "write_memory_byte", target: "wJumptableIndex", value: 0 }),
          ]),
        }),
        expect.objectContaining({
          id: "button_cancel",
          operations: [expect.objectContaining({ op: "stop_audio", audio: "MUSIC_NONE" })],
        }),
        expect.objectContaining({
          id: "cleanup",
          operations: expect.arrayContaining([
            expect.objectContaining({
              op: "restore_memory_byte",
              target: "hVBlank",
              storage: expect.objectContaining({ stack_slot: 2 }),
            }),
            expect.objectContaining({
              op: "restore_memory_byte",
              target: "hInMenu",
              storage: expect.objectContaining({ stack_slot: 1 }),
            }),
            expect.objectContaining({
              op: "restore_memory_byte",
              target: "rWBK",
              storage: expect.objectContaining({ stack_slot: 0 }),
            }),
          ]),
        }),
      ]),
      required_consumer: {
        id: "runtime_title_screen.crystal_intro",
        required: true,
      },
    });
    expect({
      reason: "missing_subprogram_contract",
      block: ".done@CrystalIntro",
      target: "ClearBGPalettes",
      opcode: "call",
      args: ["ClearBGPalettes"],
      source_span: {
        file: "engine/movie/intro.asm",
        start_line: 29,
        end_line: 29,
      },
      compiled_prefix: {
        source_entry: "CrystalIntro",
        block: "IntroScene1",
        operations: crystalIntroScenePhase?.operations,
        sprite_programs: crystalIntro?.sprite_programs,
      },
    }).toEqual({
      reason: "missing_subprogram_contract",
      block: ".done@CrystalIntro",
      target: "ClearBGPalettes",
      opcode: "call",
      args: ["ClearBGPalettes"],
      source_span: {
        file: "engine/movie/intro.asm",
        start_line: 29,
        end_line: 29,
      },
      compiled_prefix: {
        source_entry: "CrystalIntro",
        block: "IntroScene1",
        operations: expect.arrayContaining([
          expect.objectContaining({
            op: "fill_memory",
            target: "wBGPals2",
            byte_count: 128,
            value: 0,
          }),
          expect.objectContaining({
            op: "subtract_memory_byte",
            target: "hSCX",
            delta: 10,
            wrap: "u8",
          }),
          expect.objectContaining({
            op: "postincrement_memory_byte",
            target: "wIntroSceneFrameCounter",
            result: "intro_scene_frame",
          }),
          expect.objectContaining({
            op: "branch_memory_compare",
            source: "wGlobalAnimXOffset",
            predicate: "unsigned_less_than",
            operand: 0x88,
            target: ".disappear@IntroScene14",
          }),
          expect.objectContaining({
            op: "subtract_memory_byte",
            target: "wGlobalAnimXOffset",
            delta: 8,
            wrap: "u8",
          }),
          expect.objectContaining({
            op: "subtract_memory_byte",
            target: "wGlobalAnimXOffset",
            delta: 2,
            wrap: "u8",
          }),
          expect.objectContaining({
            op: "request_2bpp_transfer",
            source_symbol: "IntroGrass4GFX",
            target: "vTiles1 tile $00",
            tile_count: 1,
          }),
          expect.objectContaining({
            op: "copy_strided_memory",
            source: "wDecompressScratch",
            source_stride: 32,
            target: "wTilemap",
            target_stride: 20,
            row_count: 18,
            bytes_per_row: 20,
          }),
          expect.objectContaining({
            op: "conditional_tilemap_xor",
            clock: "wIntroSceneFrameCounter",
            clock_mask: 3,
            swap_phase: expect.objectContaining({
              equals: 0,
              byte_count: 360,
              xor: 8,
            }),
          }),
          expect.objectContaining({
            op: "add_memory_byte",
            target: "hSCY",
            delta: 8,
            wrap: "u8",
          }),
          expect.objectContaining({
            op: "return_if_memory_equal",
            source: "hSCX",
            operand: 0x60,
          }),
          expect.objectContaining({
            op: "add_memory_byte",
            target: "hSCX",
            delta: 8,
            wrap: "u8",
          }),
          expect.objectContaining({
            op: "request_2bpp_transfer",
            source_symbol: "IntroGrass4GFX",
            target: "vTiles1 tile $7f",
            tile_count: 1,
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hSCY",
            value: 216,
          }),
          expect.objectContaining({
            op: "write_memory_bytes",
            target: "wSpriteAnimDict",
            bytes: [0, 0x7f],
          }),
          expect.objectContaining({
            op: "return_if_compare",
            value: "intro_scene_frame",
            predicate: "unsigned_greater_or_equal",
            operand: 0x58,
          }),
          expect.objectContaining({
            op: "write_memory_byte_from_masked_result",
            target: "wIntroSceneTimer",
            mask: 0x1c,
            shift_right: 2,
          }),
          expect.objectContaining({
            op: "copy_indexed_palette",
            destinations: ["wBGPals2", "wBGPals1"],
            bytes_per_palette: 8,
          }),
          expect.objectContaining({
            op: "tilemap_xor",
            target: "wTilemap",
            byte_count: 360,
            xor: 8,
          }),
          expect.objectContaining({
            op: "wait_frames",
            frames: 3,
            source_span: expect.objectContaining({ start_line: 993 }),
          }),
          expect.objectContaining({
            op: "deinitialize_all_sprites",
            source_span: expect.objectContaining({ start_line: 1010 }),
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "wIntroSceneFrameCounter",
            value: 0,
            source_span: expect.objectContaining({ start_line: 1015 }),
          }),
          expect.objectContaining({
            op: "set_local_from_masked_result",
            name: "accumulator",
            source: "intro_scene_frame",
            mask: 0x1c,
            shift_left: 1,
            valid_values: [0, 8, 16, 24, 32, 40, 48, 56],
          }),
          expect.objectContaining({
            op: "broadcast_indexed_palette",
            source: "gfx/intro/fade.pal",
            bytes_per_palette: 8,
            destination: "wBGPals2",
            destination_palette_count: 8,
            behavior: "repeat_selected_palette",
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "wIntroSceneFrameCounter",
            value: 0x40,
          }),
          expect.objectContaining({
            op: "set_local_from_memory",
            name: "intro_scene_countdown",
            source: "wIntroSceneFrameCounter",
            subtract: 1,
            wrap: "u8",
          }),
          expect.objectContaining({
            op: "branch_compare",
            value: "intro_scene_countdown",
            predicate: "equal",
            operand: 0,
            target: ".done@IntroScene25",
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hCGBPalUpdate",
            value: 1,
          }),
          expect.objectContaining({
            op: "palette_transfer_request",
            condition: expect.objectContaining({
              source: "hCGB",
              predicate: "nonzero",
            }),
          }),
          expect.objectContaining({ op: "wait_frames", frames: 1 }),
          expect.objectContaining({
            op: "fill_memory",
            target: "wShadowOAM",
            byte_count: 160,
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hBGMapMode",
            value: 0,
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "rVBK",
            value: 1,
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource: "gfx/intro/unown_a.attrmap.lz",
            resource_symbol: "IntroUnownAAttrmap",
            target: "wDecompressScratch",
            output_byte_count: 1024,
          }),
          expect.objectContaining({
            op: "request_2bpp_transfer",
            target: "vBGMap0",
            target_vram_bank: 1,
            tile_count: 64,
            byte_count: 1024,
            source_segments: [
              {
                resource: "gfx/intro/unown_a.attrmap.lz",
                resource_offset: 0,
                scratch_offset: 0,
                byte_count: 1024,
              },
            ],
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource: "gfx/intro/unowns.2bpp.lz",
            resource_symbol: "IntroUnownsGFX",
            target: "wDecompressScratch",
            output_byte_count: 2048,
          }),
          expect.objectContaining({
            op: "request_2bpp_transfer",
            target: "vTiles2 tile $00",
            target_vram_bank: 0,
            tile_count: 128,
            byte_count: 2048,
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource: "gfx/intro/pulse.2bpp.lz",
            resource_symbol: "IntroPulseGFX",
            target: "wDecompressScratch",
            output_byte_count: 256,
          }),
          expect.objectContaining({
            op: "request_2bpp_transfer",
            target: "vTiles0 tile $00",
            target_vram_bank: 0,
            tile_count: 128,
            byte_count: 2048,
            source_segments: [
              {
                resource: "gfx/intro/pulse.2bpp.lz",
                resource_offset: 0,
                scratch_offset: 0,
                byte_count: 256,
              },
              {
                resource: "gfx/intro/unowns.2bpp.lz",
                resource_offset: 256,
                scratch_offset: 256,
                byte_count: 1792,
              },
            ],
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource: "gfx/intro/unown_a.tilemap.lz",
            resource_symbol: "IntroUnownATilemap",
            target: "wDecompressScratch",
            output_byte_count: 1024,
          }),
          expect.objectContaining({
            op: "request_2bpp_transfer",
            target: "vBGMap0",
            target_vram_bank: 0,
            tile_count: 64,
            byte_count: 1024,
          }),
          expect.objectContaining({
            op: "copy_memory",
            source: "IntroUnownsPalette",
            resource: "gfx/intro/unowns.pal",
            target: "wBGPals1",
            byte_count: 128,
            direction: "ascending",
            destination_segments: [
              { target: "wBGPals1", target_offset: 0, byte_count: 64 },
              { target: "wOBPals1", target_offset: 0, byte_count: 64 },
            ],
          }),
          expect.objectContaining({
            op: "copy_memory",
            source: "IntroUnownsPalette",
            resource: "gfx/intro/unowns.pal",
            target: "wBGPals2",
            byte_count: 128,
            direction: "ascending",
            destination_segments: [
              { target: "wBGPals2", target_offset: 0, byte_count: 64 },
              { target: "wOBPals2", target_offset: 0, byte_count: 64 },
            ],
          }),
          expect.objectContaining({
            op: "restore_memory_byte",
            target: "rWBK",
            storage: { kind: "cpu_stack", register_pair: "af" },
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hSCX",
            value: 0,
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hSCY",
            value: 0,
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hWX",
            value: 7,
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hWY",
            value: 144,
          }),
          expect.objectContaining({
            op: "fill_memory",
            target: "wSpriteAnimData",
            target_end_exclusive: "wSpriteAnimDataEnd",
            byte_count: 193,
            value: 0,
            direction: "ascending",
            bank: { select: "wram0", restore: false },
            destination_views: [
              {
                id: "sprite_animation",
                byte_offset: 0,
                byte_count: 180,
              },
              {
                id: "mobile_union_alias",
                byte_offset: 0,
                byte_count: 180,
              },
              {
                id: "scheduler_state",
                byte_offset: 180,
                byte_count: 13,
              },
            ],
            destination_labels: expect.arrayContaining([
              "wSpriteAnimDict",
              "wSpriteAnimationStructs",
              "wMobileWRAM",
              "wSpriteAnimCount",
              "wSpriteAnimAddrBackup",
              "wGlobalAnimXOffset",
            ]),
            invocation: expect.objectContaining({
              call_form: "farcall",
              target: "ClearSpriteAnims",
              target_bank: "BANK(ClearSpriteAnims)",
              restores_rom_bank: true,
              preserves_callee_bc: true,
              scratch_writes: [
                {
                  target: "hTempBank",
                  value: "BANK(ClearSpriteAnims)",
                },
                { target: "wFarCallBC", value: [0, 0] },
              ],
              register_result: {
                a: 0,
                bc: 0,
                hl: "wSpriteAnimDataEnd",
                de: "unchanged_by_callee",
                flags: {
                  zero: true,
                  subtract: false,
                  half_carry: false,
                  carry: false,
                },
              },
              source_span: {
                file: "engine/movie/intro.asm",
                start_line: 140,
                end_line: 140,
              },
              macro_source_span: {
                file: "macros/farcall.asm",
                start_line: 7,
                end_line: 11,
              },
            }),
            implementation_source_span: {
              file: "engine/sprite_anims/core.asm",
              start_line: 1,
              end_line: 11,
            },
            value_source_span: {
              file: "engine/sprite_anims/core.asm",
              start_line: 5,
              end_line: 5,
            },
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 140,
              end_line: 140,
            },
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hCGBPalUpdate",
            value: 1,
            address_space: "hram",
            implementation_source_span: {
              file: "engine/movie/intro.asm",
              start_line: 1549,
              end_line: 1552,
            },
            value_source_span: {
              file: "constants/misc_constants.asm",
              start_line: 3,
              end_line: 3,
            },
            target_declaration_source_span: {
              file: "ram/hram.asm",
              start_line: 165,
              end_line: 165,
            },
            invocation: {
              call_form: "call",
              target: "Intro_SetCGBPalUpdate",
              stack_effect: "push_return_address_then_ret",
              register_result: {
                a: 1,
                bc: "unchanged",
                de: "unchanged",
                hl: "unchanged",
                flags: "unchanged",
              },
              source_span: {
                file: "engine/movie/intro.asm",
                start_line: 141,
                end_line: 141,
              },
            },
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 141,
              end_line: 141,
            },
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "wIntroSceneFrameCounter",
            value: 0,
            address_space: "wram",
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 142,
              end_line: 143,
            },
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "wIntroSceneTimer",
            value: 0,
            address_space: "wram",
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 142,
              end_line: 144,
            },
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource_symbol: "IntroCrystalUnownsAttrmap",
            output_byte_count: 1024,
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource_symbol: "IntroCrystalUnownsGFX",
            output_byte_count: 512,
          }),
          expect.objectContaining({
            op: "decompress_lz3_resource",
            resource_symbol: "IntroCrystalUnownsTilemap",
            output_byte_count: 1024,
          }),
          expect.objectContaining({
            op: "fade_unown_word_palettes",
            palette_index: {
              source: "accumulator",
              multiply: 8,
              valid_values: [0, 1, 2, 3, 4, 5, 6, 7],
            },
            fade_index: {
              source: "wIntroSceneTimer",
              multiply: 2,
              valid_values: Array.from({ length: 16 }, (_, index) => index),
            },
            target: "wBGPals2",
            target_color_offsets: [4, 6],
            fast_hues: [31, 30, 28, 27, 25, 24, 22, 21, 19, 18, 16, 15, 13, 12, 10, 9],
            slow_hues: Array.from({ length: 16 }, (_, index) => 31 - index),
            color_encoding: "rgb555_grayscale",
            completion_write: { target: "hCGBPalUpdate", value: 1 },
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "wIntroSceneFrameCounter",
            value: 0x80,
          }),
          expect.objectContaining({
            op: "decrement_memory_byte",
            target: "wIntroSceneFrameCounter",
            comparison_value: "predecrement_value",
          }),
          expect.objectContaining({
            op: "play_audio",
            audio: "SFX_INTRO_WHOOSH",
          }),
          expect.objectContaining({
            op: "set_memory_bit",
            target: "wJumptableIndex",
            bit: 7,
          }),
          expect.objectContaining({
            op: "increment_memory_byte",
            target: "wJumptableIndex",
            delta: 1,
            wrap: "u8",
            target_declaration_source_span: {
              file: "ram/wram.asm",
              start_line: 1510,
              end_line: 1510,
            },
            target_section_source_span: {
              file: "ram/wram.asm",
              start_line: 1299,
              end_line: 1299,
            },
            implementation_source_span: {
              file: "engine/movie/intro.asm",
              start_line: 91,
              end_line: 94,
            },
            invocation: expect.objectContaining({
              call_form: "call",
              target: "NextIntroScene",
            }),
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 145,
              end_line: 145,
            },
          }),
          {
            op: "return",
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 146,
              end_line: 146,
            },
          },
          expect.objectContaining({
            op: "sprite_scheduler_step",
            instances: expect.arrayContaining([
              expect.stringContaining("engine/movie/intro.asm"),
            ]),
            instance_activation: expect.arrayContaining([
              expect.objectContaining({
                instance: expect.stringContaining("engine/movie/intro.asm"),
                lifetime: expect.objectContaining({
                  callback_before_frame_update: true,
                }),
              }),
            ]),
            struct_slots: 10,
            callback_before_frame_update: true,
            oam_cursor: {
              target: "wCurSpriteOAMAddr",
              value: "LOW(wShadowOAM)",
            },
            remaining_oam_clear: {
              target: "wShadowOAM",
              target_end_exclusive: "wShadowOAMEnd",
              byte_count: 160,
              value: 0,
              direction: "ascending",
            },
            invocation: expect.objectContaining({
              call_form: "farcall",
              target: "PlaySpriteAnimations",
              target_bank: "BANK(PlaySpriteAnimations)",
              scratch_writes: [
                {
                  target: "hTempBank",
                  value: "BANK(PlaySpriteAnimations)",
                },
                { target: "wFarCallBC", value: [0, 0] },
              ],
              register_result: {
                a: 0,
                bc: 0,
                hl: "PlaySpriteAnimations",
                de: "unchanged_by_callee",
                flags: {
                  zero: false,
                  subtract: false,
                  half_carry: false,
                  carry: false,
                },
              },
            }),
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 20,
              end_line: 20,
            },
          }),
          expect.objectContaining({
            op: "wait_frames",
            frames: 1,
            condition: {
              source: null,
              predicate: "always",
              source_span: null,
            },
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 21,
              end_line: 21,
            },
          }),
          {
            op: "jump",
            target: ".loop@CrystalIntro",
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 22,
              end_line: 22,
            },
          },
          expect.objectContaining({
            op: "sample_input",
            routine: "JoyTextDelay",
            sampler: "GetJoypad",
            result: "hJoyLast",
            menu_guard: "hInMenu",
            menu_zero_source: "hJoyPressed",
            menu_nonzero_source: "hJoyDown",
            repeat_delay: "wTextDelayFrames",
            pressed_repeat_reset: 15,
            idle_repeat_restart: 5,
            invocation: expect.objectContaining({
              call_form: "call",
              target: "JoyTextDelay",
              source_span: {
                file: "engine/movie/intro.asm",
                start_line: 12,
                end_line: 12,
              },
            }),
          }),
          expect.objectContaining({
            op: "input_branch",
            sample: "hJoyLast",
            require_all: [],
            require_any: [
              expect.objectContaining({ symbol: "PAD_BUTTONS", value: 15 }),
            ],
            forbid_any: [],
            target: ".ShutOffMusic@CrystalIntro",
          }),
          expect.objectContaining({
            op: "memory_branch",
            source: "wJumptableIndex",
            predicate: "bit_set",
            bit: expect.objectContaining({
              symbol: "JUMPTABLE_EXIT_F",
              value: 7,
            }),
            target: ".done@CrystalIntro",
          }),
          expect.objectContaining({
            op: "dispatch_table",
            dispatcher: "IntroSceneJumper",
            table: "IntroScenes",
            index: "wJumptableIndex",
            entries: Array.from(
              { length: 28 },
              (_, index) => `IntroScene${index + 1}`,
            ),
            domain: {
              minimum: 0,
              maximum: 27,
              values: Array.from({ length: 28 }, (_, index) => index),
            },
          }),
          expect.objectContaining({
            op: "postincrement_memory_byte",
            target: "wIntroSceneFrameCounter",
            result: "intro_scene_frame",
            delta: 1,
            wrap: "u8",
          }),
          expect.objectContaining({
            op: "branch_compare",
            value: "intro_scene_frame",
            predicate: "unsigned_greater_or_equal",
            operand: 128,
            target: ".endscene@IntroScene2",
          }),
          expect.objectContaining({
            op: "branch_compare",
            value: "intro_scene_frame",
            predicate: "not_equal",
            operand: 96,
            target: ".nosound@IntroScene2",
          }),
          expect.objectContaining({
            op: "sprite_init_group",
            instances: expect.any(Array),
            origin: { macro: "depixel", x: 11, y: 11 },
            preserves: "intro_scene_frame",
          }),
          expect.objectContaining({
            op: "sprite_activate",
            object: expect.objectContaining({
              symbol: "SPRITE_ANIM_OBJ_INTRO_SUICUNE",
            }),
            origin: {
              macro: "depixel",
              x_tile: 13,
              y_tile: 27,
              x_pixel: 4,
              y_pixel: 0,
              x: 108,
              y: 216,
            },
          }),
          expect.objectContaining({
            op: "transform_memory_byte",
            target: "wGlobalAnimXOffset",
            input: "global_anim_x",
            operator: "subtract",
            operand: 8,
            wrap: "u8",
          }),
          expect.objectContaining({
            op: "deinitialize_all_sprites",
            struct_indices: Array.from({ length: 10 }, (_, index) => index),
          }),
          expect.objectContaining({
            op: "fill_memory",
            target: "wAttrmap",
            target_offset: 0,
            byte_count: 240,
            value: 1,
          }),
          expect.objectContaining({
            op: "fill_memory",
            target: "wAttrmap",
            target_offset: 240,
            byte_count: 60,
            value: 2,
          }),
          expect.objectContaining({
            op: "fill_memory",
            target: "wAttrmap",
            target_offset: 300,
            byte_count: 60,
            value: 3,
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hBGMapAddress",
            value: 0x0c,
          }),
          expect.objectContaining({
            op: "indexed_2bpp_request",
            condition: expect.objectContaining({
              source: "wIntroSceneFrameCounter",
              predicate: "unsigned_less_than",
              operand: 36,
            }),
            selector: {
              source: "wIntroSceneFrameCounter",
              mask: 0x0c,
              shift_right: 1,
              byte_offsets: [0, 2, 4, 6],
            },
            target: "vTiles2 tile $09",
            tile_count: 4,
            byte_count: 64,
          }),
          expect.objectContaining({
            op: "sprite_activate",
            object: expect.objectContaining({
              symbol: "SPRITE_ANIM_OBJ_INTRO_PICHU",
            }),
            origin: expect.objectContaining({ x: 169, y: 128 }),
          }),
          expect.objectContaining({
            op: "sprite_activate",
            object: expect.objectContaining({
              symbol: "SPRITE_ANIM_OBJ_INTRO_WOOPER",
            }),
            origin: expect.objectContaining({ x: 176, y: 48 }),
          }),
          expect.objectContaining({
            op: "scheduled_audio",
            clock: "wIntroSceneFrameCounter",
            sentinel: 0xff,
            entries: [
              { frame: 0x00, audio: "SFX_INTRO_UNOWN_3" },
              { frame: 0x20, audio: "SFX_INTRO_UNOWN_2" },
              { frame: 0x40, audio: "SFX_INTRO_UNOWN_1" },
              { frame: 0x60, audio: "SFX_INTRO_UNOWN_2" },
              { frame: 0x80, audio: "SFX_INTRO_UNOWN_3" },
              { frame: 0x90, audio: "SFX_INTRO_UNOWN_2" },
              { frame: 0xa0, audio: "SFX_INTRO_UNOWN_1" },
              { frame: 0xb0, audio: "SFX_INTRO_UNOWN_2" },
            ],
            on_match: {
              stop_sfx_channels: [5, 6, 7, 8],
              play_entry: true,
            },
          }),
          expect.objectContaining({
            op: "compute_byte",
            input: "intro_scene_frame",
            steps: [
              { op: "mask", value: 0x0f },
              { op: "shift_left", value: 1 },
              { op: "shift_left", value: 1 },
            ],
            result: "intro_scene_timer_value",
          }),
          expect.objectContaining({
            op: "play_audio",
            audio: "SFX_INTRO_UNOWN_1",
          }),
          expect.objectContaining({
            op: "write_memory_byte_from_result",
            target: "wIntroSceneTimer",
            result: "intro_scene_frame",
          }),
          expect.objectContaining({
            op: "set_local",
            name: "accumulator",
            value: 0,
          }),
          expect.objectContaining({
            op: "palette_fade_lookup",
            palette_selector: "accumulator",
            selector_stride: 8,
            first_color_offset: 2,
            timer: expect.objectContaining({
              source: "wIntroSceneTimer",
              mask: 63,
              fold_above: 31,
              fold_from: 63,
            }),
            clear: { target: "wBGPals2", byte_count: 64, value: 0 },
            tables: [
              expect.objectContaining({ label: ".BWFade", colors: expect.any(Array) }),
              expect.objectContaining({ label: ".BlackLBlueFade", colors: expect.any(Array) }),
              expect.objectContaining({ label: ".BlackBlueFade", colors: expect.any(Array) }),
            ],
            writes: [
              { target_offset: 2, table: ".BWFade", encoding: "rgb555_little_endian" },
              { target_offset: 4, table: ".BlackLBlueFade", encoding: "rgb555_little_endian" },
              { target_offset: 6, table: ".BlackBlueFade", encoding: "rgb555_little_endian" },
            ],
            transfer_request: { target: "hCGBPalUpdate", value: 1 },
          }),
          expect.objectContaining({
            op: "fill_memory",
            target: "wLYOverrides",
            byte_count: 144,
            value: 0,
            bank: { select: "BANK(wLYOverrides)", restore: true },
          }),
          expect.objectContaining({
            op: "write_memory_byte",
            target: "hLCDCPointer",
            value: 0x43,
          }),
          expect.objectContaining({
            op: "perspective_scroll",
            target: "wLYOverrides",
            byte_count: 144,
            frame: expect.objectContaining({
              source: "wIntroSceneFrameCounter",
              parity_mask: 1,
            }),
            bands: [
              expect.objectContaining({
                id: "trees",
                offset: 0,
                byte_count: 0x5f,
                delta: 1,
                cadence: "odd_frames",
              }),
              expect.objectContaining({
                id: "grass",
                offset: 0x5f,
                byte_count: 0x31,
                delta: 2,
                cadence: "every_frame",
              }),
            ],
            horizontal_scroll: expect.objectContaining({
              source_offset: 0,
              target: "hSCX",
            }),
          }),
          expect.objectContaining({
            op: "branch_compare",
            value: "intro_scene_frame",
            predicate: "equal",
            operand: 0x80,
            target: ".endscene@IntroScene4",
          }),
          expect.objectContaining({
            op: "branch_compare",
            value: "intro_scene_frame",
            predicate: "equal",
            operand: 0x60,
            target: ".SecondUnown@IntroScene6",
          }),
          expect.objectContaining({
            op: "branch_compare",
            value: "intro_scene_frame",
            predicate: "unsigned_greater_or_equal",
            operand: 0x40,
            target: ".StopUnown@IntroScene6",
          }),
          expect.objectContaining({
            op: "sprite_init_group",
            origin: { macro: "depixel", x: 7, y: 15 },
            preserves: "intro_scene_frame",
          }),
          expect.objectContaining({
            op: "sprite_init_group",
            origin: { macro: "depixel", x: 14, y: 6 },
            preserves: "intro_scene_frame",
          }),
        ]),
        sprite_programs: expect.arrayContaining([
          expect.objectContaining({
            initializer_source_span: expect.objectContaining({
              file: "engine/movie/intro.asm",
            }),
          }),
        ]),
      },
    });
    const introSpriteByAllocationLine = (line: number) =>
      crystalIntro?.sprite_programs.find(
        (sprite) => sprite.allocation_source_span?.start_line === line,
      );
    expect(introSpriteByAllocationLine(393)?.graphic_binding).toMatchObject({
      resource: "gfx/intro/suicune_run.2bpp.lz",
      target_vram_bank: 0,
      tile_base: 0,
    });
    expect(introSpriteByAllocationLine(486)?.graphic_binding).toMatchObject({
      resource: "gfx/intro/pichu_wooper.2bpp.lz",
      target_vram_bank: 1,
      tile_base: 0,
    });
    expect(introSpriteByAllocationLine(784)?.graphic_binding).toMatchObject({
      resource: "gfx/intro/unown_back.2bpp.lz",
      target_vram_bank: 0,
      tile_base: 0,
    });
    expect(introSpriteByAllocationLine(787)?.graphic_binding).toMatchObject({
      resource: "gfx/intro/grass4.2bpp",
      target_vram_bank: 0,
      tile_base: 0x80,
    });
    expect(introSpriteByAllocationLine(936)?.graphic_binding).toMatchObject({
      resource: "gfx/intro/grass4.2bpp",
      target_vram_bank: 0,
      tile_base: 0xff,
    });
    const introPrefix = crystalIntroScenePhase?.operations ?? [];
    const backgroundBindings = introPrefix.filter(
      (operation) => operation.op === "intro_background_binding",
    );
    expect(backgroundBindings).toHaveLength(28);
    expect(backgroundBindings.find((binding) => binding.dispatcher_entry === 14)).toMatchObject({
      tilemap_resource: "gfx/intro/suicune_jump.tilemap.lz",
      attrmap_resource: "gfx/intro/suicune_jump.attrmap.lz",
      palette_resource: "gfx/intro/suicune.pal",
      tile_bindings: expect.arrayContaining([
        expect.objectContaining({ resource: "gfx/intro/suicune_jump.2bpp.lz" }),
      ]),
    });
    expect(backgroundBindings.find((binding) => binding.dispatcher_entry === 18)).toMatchObject({
      tilemap_resource: "gfx/intro/suicune_back.tilemap.lz",
      attrmap_resource: "gfx/intro/suicune_back.attrmap.lz",
      palette_resource: "gfx/intro/suicune.pal",
      tile_bindings: expect.arrayContaining([
        expect.objectContaining({ resource: "gfx/intro/suicune_back.2bpp.lz" }),
        expect.objectContaining({ resource: "gfx/intro/unowns.2bpp.lz" }),
      ]),
    });
    const unownFade = introPrefix.find(
      (operation) => operation.op === "palette_fade_lookup",
    );
    expect(unownFade).toMatchObject({
      tables: [
        { colors: expect.arrayContaining([0, 16912, 32767]) },
        { colors: expect.arrayContaining([0, 16640, 32224]) },
        { colors: expect.arrayContaining([0, 16384, 31744]) },
      ],
    });
    expect(
      introPrefix.find(
        (operation) =>
          operation.op === "request_2bpp_transfer" &&
          operation.tile_count === 255,
      ),
    ).toMatchObject({
      byte_count: 4080,
      source_segments: [
        {
          resource: "gfx/intro/suicune_run.2bpp.lz",
          resource_offset: 0,
          scratch_offset: 0,
          byte_count: 3072,
        },
        {
          resource: "wDecompressScratch",
          resource_offset: 3072,
          scratch_offset: 3072,
          byte_count: 1008,
          origin: "preexisting_memory",
        },
      ],
    });
    expect(
      introPrefix.find(
        (operation) =>
          operation.op === "write_memory_byte" &&
          operation.target === "hCGBPalUpdate",
      ),
    ).toMatchObject({
      value: 1,
      condition: {
        source: null,
        predicate: "always",
        source_span: null,
      },
    });
    expect(
      introPrefix.find(
        (operation) => operation.op === "palette_transfer_request",
      ),
    ).toMatchObject({
      condition: { source: "hCGB", predicate: "nonzero" },
      request: {
        target: "hCGBPalUpdate",
        queued_value: 1,
        completion_value: 0,
      },
      schedule: "vblank",
    });
    expect(
      introPrefix.filter(
        (operation) =>
          operation.op === "wait_frames" &&
          operation.source_span.file === "engine/movie/intro.asm",
      ),
    ).toEqual([
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1569 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1570 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 21 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1569 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1570 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1569 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1570 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1569 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1570 }),
      }),
      expect.objectContaining({
        frames: 3,
        source_span: expect.objectContaining({ start_line: 452 }),
      }),
      expect.objectContaining({
        frames: 3,
        source_span: expect.objectContaining({ start_line: 457 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1569 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1570 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1569 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1570 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1569 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1570 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1569 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1570 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1569 }),
      }),
      expect.objectContaining({
        frames: 1,
        source_span: expect.objectContaining({ start_line: 1570 }),
      }),
      expect.objectContaining({
        frames: 3,
        source_span: expect.objectContaining({ start_line: 993 }),
      }),
    ]);

    const subprograms = (checkpoint as any).subprograms as Array<any>;
    expect(subprograms).toHaveLength(15);
    const splashSubprogram = subprograms.find(
      (subprogram) => subprogram.id === "splash_screen",
    );
    expect(splashSubprogram).toMatchObject({
      id: "splash_screen",
      source_entry: "SplashScreen",
      accepted_call_forms: ["callfar"],
      result: {
        name: "splash_outcome",
        storage: "carry",
        domain: expect.arrayContaining([
          expect.objectContaining({
            id: "cancelled",
            value: 1,
            condition: expect.objectContaining({
              kind: "masked_input_nonzero",
              source: "hJoyLast",
              mask: { symbol: "PAD_BUTTONS", value: 0x0f },
            }),
          }),
          expect.objectContaining({
            id: "completed",
            value: 0,
            condition: expect.objectContaining({
              kind: "memory_bit_set",
              source: "wJumptableIndex",
              bit: { symbol: "JUMPTABLE_EXIT_F", value: 7 },
            }),
          }),
          expect.objectContaining({
            id: "non_returning_dmg",
            value: null,
            condition: expect.objectContaining({
              kind: "memory_zero",
              source: "hCGB",
            }),
          }),
        ]),
      },
      phases: expect.arrayContaining([
        expect.objectContaining({
          id: "reset",
          operations: expect.arrayContaining([
            expect.objectContaining({
              op: "stop_audio",
              audio: "MUSIC_NONE",
            }),
            expect.objectContaining({
              op: "write_memory_byte",
              target: "hBGMapAddress + 1",
              value: "HIGH(vBGMap0)",
            }),
            expect.objectContaining({
              op: "write_memory_byte",
              target: "hWY",
              value: 144,
            }),
            expect.objectContaining({
              op: "apply_palette_layout",
              layout: { symbol: "SCGB_GAMEFREAK_LOGO", value: 25 },
              branches: expect.arrayContaining([
                expect.objectContaining({
                  id: "sgb",
                  operations: [
                    expect.objectContaining({
                      op: "sgb_packet_transfer",
                      packet: "PalPacket_GamefreakLogo",
                    }),
                    expect.objectContaining({
                      op: "sgb_packet_transfer",
                      packet: "BlkPacket_AllPal0",
                    }),
                  ],
                }),
                expect.objectContaining({
                  id: "cgb",
                  operations: expect.arrayContaining([
                    expect.objectContaining({
                      op: "copy_palette",
                      target: "wBGPals1 palette 0",
                    }),
                    expect.objectContaining({
                      op: "copy_palette",
                      target: "wOBPals1 palette 1",
                    }),
                    expect.objectContaining({
                      op: "upload_attrmap",
                      width: 20,
                      height: 18,
                      target_stride: 32,
                    }),
                  ]),
                }),
              ]),
            }),
            expect.objectContaining({ op: "wait_frames", frames: 10 }),
          ]),
        }),
        expect.objectContaining({
          id: "copyright",
          operations: expect.arrayContaining([
            expect.objectContaining({ op: "wait_frames", frames: 100 }),
          ]),
        }),
        expect.objectContaining({
          id: "teardown",
          operations: expect.arrayContaining([
            expect.objectContaining({
              op: "fill_memory",
              target: "wShadowOAM",
              value: 0,
            }),
            expect.objectContaining({ op: "wait_frames", frames: 16 }),
          ]),
        }),
      ]),
      loop: {
        source_span: { file: splashPath, start_line: 38, end_line: 49 },
        order: [
          "sample_input",
          "cancel_if_buttons",
          "test_exit",
          "dispatch_scene",
          "sprite_scheduler_step",
          "wait_frame",
          "repeat",
        ],
        input: expect.objectContaining({
          routine: "JoyTextDelay",
          result: "hJoyLast",
          menu_zero_source: "hJoyPressed",
          menu_nonzero_source: "hJoyDown",
          repeat_delay: "wTextDelayFrames",
          mask: { symbol: "PAD_BUTTONS", value: 0x0f },
        }),
        scene_dispatch: expect.objectContaining({
          table: ".scenes@GameFreakPresentsScene",
          index: "wJumptableIndex",
          domain: { minimum: 0, maximum: 3, values: [0, 1, 2, 3] },
        }),
        natural_scheduler_ticks: 376,
        scheduler: expect.objectContaining({
          op: "sprite_scheduler_step",
          instances: ["sprite:engine/movie/splash.asm:93"],
        }),
        frame_wait: expect.objectContaining({
          op: "wait_frames",
          frames: 1,
        }),
      },
      resource_transfers: expect.arrayContaining([
        expect.objectContaining({
          id: "font_extra_solid_black",
          resources: ["gfx/font/black.1bpp"],
          encoding: "1bpp",
          target: "vTiles2 tile '■'",
          tile_count: 1,
        }),
        expect.objectContaining({
          id: "font_extra_phone_icon",
          resources: ["gfx/font/phone_icon.2bpp"],
          encoding: "2bpp",
          target: "vTiles2 tile '☎'",
          tile_count: 1,
        }),
        expect.objectContaining({
          id: "font_extra_glyphs",
          resources: ["gfx/font/font_extra.2bpp"],
          encoding: "2bpp",
          source: "FontExtra + 3 tiles",
          target: "vTiles2 tile '<BOLD_D>'",
          tile_count: 22,
        }),
        expect.objectContaining({
          id: "font_extra_selected_frame",
          resources: [
            "gfx/frames/1.1bpp",
            "gfx/frames/2.1bpp",
            "gfx/frames/3.1bpp",
            "gfx/frames/4.1bpp",
            "gfx/frames/5.1bpp",
            "gfx/frames/6.1bpp",
            "gfx/frames/7.1bpp",
            "gfx/frames/8.1bpp",
          ],
          encoding: "1bpp",
          selector: expect.objectContaining({
            source: "wTextboxFrame",
            domain: [0, 1, 2, 3, 4, 5, 6, 7],
          }),
          target: "vTiles2 tile '┌'",
          tile_count: 6,
        }),
        expect.objectContaining({
          resources: ["gfx/splash/copyright.2bpp"],
          encoding: "2bpp",
          target: "vTiles2 tile $60",
          tile_count: 29,
        }),
        expect.objectContaining({
          resources: [
            "gfx/splash/gamefreak_presents.1bpp",
            "gfx/splash/gamefreak_logo.1bpp",
          ],
          encoding: "1bpp",
          target: "vTiles2",
          tile_count: 28,
        }),
        expect.objectContaining({
          resources: ["gfx/splash/ditto.2bpp.lz"],
          encoding: "lz_2bpp",
          target: "wDecompressScratch",
        }),
        expect.objectContaining({
          resources: ["gfx/sgb/gbc_only.2bpp.lz"],
          encoding: "lz_2bpp",
          target: "wGBCOnlyDecompressBuffer",
          condition: { source: "hCGB", predicate: "zero" },
        }),
      ]),
      resources: expect.arrayContaining([
        expect.objectContaining({
          path: "gfx/sgb/predef.pal",
          kind: "palette",
        }),
      ]),
      tilemap_writes: expect.arrayContaining([
        expect.objectContaining({
          target: "wTilemap coord 5,10",
          bytes: [0, 1, 2, 3, 13, 4, 5, 3, 1, 6],
          dispatch_tick: 182,
        }),
        expect.objectContaining({
          target: "wTilemap coord 7,11",
          bytes: [7, 8, 9, 10, 11, 12],
          dispatch_tick: 247,
        }),
      ]),
      sprite_operations: expect.arrayContaining([
        expect.objectContaining({
          op: "sprite_init",
          instance: "sprite:engine/movie/splash.asm:93",
        }),
        expect.objectContaining({
          op: "sprite_scheduler_step",
          instances: ["sprite:engine/movie/splash.asm:93"],
        }),
      ]),
      sprite_programs: [
        expect.objectContaining({
          instance: "sprite:engine/movie/splash.asm:93",
          object: expect.objectContaining({
            symbol: "SPRITE_ANIM_OBJ_GAMEFREAK_LOGO",
            value: 3,
          }),
        }),
      ],
      audio: expect.arrayContaining([
        expect.objectContaining({ id: "MUSIC_NONE", kind: "silence" }),
        expect.objectContaining({
          id: "SFX_GAME_FREAK_PRESENTS",
          kind: "sound_effect",
        }),
        expect.objectContaining({
          id: "SFX_DITTO_TRANSFORM",
          kind: "sound_effect",
        }),
      ]),
      required_consumer: {
        id: "runtime_title_screen.splash_screen",
        required: true,
      },
    });
  });

  it("derives the unsupported effect frontier from the exact source operand", () => {
    const mutatedIntro = replaceExact(
      canonicalRead(introPath),
      "GameInit::\n\tfarcall TryLoadSaveData",
      "GameInit::\n\tfarcall TryLoadSaveDataMutated",
    );
    const checkpoint = analyzeRuntimeTitlePresentationEmission({
      disassemblyRoot,
      audioAssetIds: new Set(),
      runtimeSpawnIdentifiers: new Set([0]),
      readSource: (relativePath) =>
        relativePath === introPath ? mutatedIntro : canonicalRead(relativePath),
    });
    expect(checkpoint.frontier).toMatchObject({
      reason: "missing_host_effect_contract",
      target: "TryLoadSaveDataMutated",
      source_span: { file: introPath, start_line: 1330, end_line: 1330 },
    });
  });

  it("builds a closed source program through the common overworld loop", () => {
    const program = buildRuntimeTitlePresentationProgram({
      disassemblyRoot,
      audioAssetIds: runtimePresentationAudioIds,
      runtimeSpawnIdentifiers: new Set([0]),
    });
    expect(program.blocks[".loop@FinishContinueFunction"]?.operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ op: "prepare_overworld_session" }),
        expect.objectContaining({ op: "run_overworld_loop" }),
        expect.objectContaining({ op: "handle_overworld_return" }),
      ]),
    );
    expect(program.resources.length).toBeGreaterThan(0);
    expect(program.audio).toHaveLength(runtimePresentationAudioIds.size);
    expect(
      program.subprograms
        .find((subprogram) => subprogram.id === "start_title_screen")
        ?.phases.find((phase) => phase.id === "title_screen")
        ?.labels,
    ).toEqual({
      TitleScreenEntrance: 68,
      ".loop@TitleScreenEntrance": 70,
      ".done@TitleScreenEntrance": 74,
      TitleScreenTimer: 79,
      TitleScreenMain: 82,
      ".check_clock_reset@TitleScreenMain": 89,
      ".check_start@TitleScreenMain": 92,
      ".incave@TitleScreenMain": 94,
      ".delete_save_data@TitleScreenMain": 94,
      ".done@TitleScreenMain": 95,
      ".end@TitleScreenMain": 97,
      ".reset_clock@TitleScreenMain": 101,
      TitleScreenEnd: 104,
    });
    const missingBranchLabel = structuredClone(program);
    const titlePhase = missingBranchLabel.subprograms
      .find((subprogram) => subprogram.id === "start_title_screen")
      ?.phases.find((phase) => phase.id === "title_screen");
    if (!titlePhase?.labels) throw new Error("test title phase labels are missing");
    delete titlePhase.labels[".check_start@TitleScreenMain"];
    expect(() => assertRuntimePresentationProgram(missingBranchLabel)).toThrow(
      /targets missing label \.check_start@TitleScreenMain/,
    );
    const missingMainMenuBranchLabel = structuredClone(program);
    const mainMenuPhase = missingMainMenuBranchLabel.subprograms
      .find((subprogram) => subprogram.id === "main_menu")
      ?.phases.find((phase) => phase.id === "main_menu");
    if (!mainMenuPhase?.labels)
      throw new Error("test main-menu phase labels are missing");
    delete mainMenuPhase.labels[".quit@MainMenu"];
    expect(() =>
      assertRuntimePresentationProgram(missingMainMenuBranchLabel),
    ).toThrow(/targets missing label \.quit@MainMenu/);
    const missingDispatchLabel = structuredClone(program);
    const dispatchPhase = missingDispatchLabel.subprograms
      .find((subprogram) => subprogram.id === "start_title_screen")
      ?.phases.find((phase) => phase.id === "title_screen");
    if (!dispatchPhase?.labels) throw new Error("test title phase labels are missing");
    delete dispatchPhase.labels.TitleScreenEnd;
    expect(() => assertRuntimePresentationProgram(missingDispatchLabel)).toThrow(
      /dispatches to missing label TitleScreenEnd/,
    );
    const missingOptionSource = structuredClone(program);
    const optionPhase = missingOptionSource.subprograms
      .find((subprogram) => subprogram.id === "start_title_screen")
      ?.phases.find((phase) => phase.id === "title_screen");
    const option = optionPhase?.operations.find(
      (operation) => operation.op === "select_title_option",
    );
    if (!option) throw new Error("test title option operation is missing");
    (option.options as Array<{ source: string }>)[0].source =
      ".missing@TitleScreenMain";
    expect(() => assertRuntimePresentationProgram(missingOptionSource)).toThrow(
      /invalid title option source \.missing@TitleScreenMain/,
    );
    const invalidOptionValue = structuredClone(program);
    const invalidOptionPhase = invalidOptionValue.subprograms
      .find((subprogram) => subprogram.id === "start_title_screen")
      ?.phases.find((phase) => phase.id === "title_screen");
    const invalidOption = invalidOptionPhase?.operations.find(
      (operation) => operation.op === "select_title_option",
    );
    if (!invalidOption) throw new Error("test title option operation is missing");
    (invalidOption.options as Array<{ value: unknown }>)[0].value = "MAIN_MENU";
    expect(() => assertRuntimePresentationProgram(invalidOptionValue)).toThrow(
      /invalid title option source .* or value MAIN_MENU/,
    );
    expect(program.text).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: "OakText1" }),
        expect.objectContaining({ id: "PasswordAskResetClockText" }),
        expect.objectContaining({ id: "ClockHasResetText" }),
      ]),
    );
  });

  it.each([
    {
      name: "the end threshold changes",
      before:
        "IntroScene2:\n; First Unown (A) fades in, pulses, then fades out.\n\tld hl, wIntroSceneFrameCounter\n\tld a, [hl]\n\tinc [hl]\n\tcp $80\n\tjr nc, .endscene",
      after:
        "IntroScene2:\n; First Unown (A) fades in, pulses, then fades out.\n\tld hl, wIntroSceneFrameCounter\n\tld a, [hl]\n\tinc [hl]\n\tcp $81\n\tjr nc, .endscene",
      diagnostic: /IntroScene2.*counter|threshold|\$80/i,
    },
    {
      name: "the Unown trigger threshold changes",
      before: "\tjr nc, .endscene\n\tcp $60\n\tjr nz, .nosound",
      after: "\tjr nc, .endscene\n\tcp $61\n\tjr nz, .nosound",
      diagnostic: /IntroScene2.*counter|threshold|\$60/i,
    },
    {
      name: "the Unown sound changes",
      before:
        "\tjr nz, .nosound\n\tpush af\n\tdepixel 11, 11\n\tcall CrystalIntro_InitUnownAnim\n\tld de, SFX_INTRO_UNOWN_1\n\tcall PlaySFX",
      after:
        "\tjr nz, .nosound\n\tpush af\n\tdepixel 11, 11\n\tcall CrystalIntro_InitUnownAnim\n\tld de, SFX_INTRO_UNOWN_2\n\tcall PlaySFX",
      diagnostic: /IntroScene2.*Unown trigger|SFX_INTRO_UNOWN_1/i,
    },
    {
      name: "the Unown fade timer mask changes",
      before:
        "\tld a, [wIntroSceneTimer]\n\tand %111111\n\tcp %011111",
      after:
        "\tld a, [wIntroSceneTimer]\n\tand %011111\n\tcp %011111",
      diagnostic: /CrystalIntro_UnownFade.*timer fold|%111111/i,
    },
    {
      name: "the Unown fade clears seven palettes",
      before:
        "\tld hl, wBGPals2\n\tld bc, 8 palettes\n\txor a\n\tcall ByteFill",
      after:
        "\tld hl, wBGPals2\n\tld bc, 7 palettes\n\txor a\n\tcall ByteFill",
      diagnostic: /CrystalIntro_UnownFade.*clear|8 palettes/i,
    },
    {
      name: "the light-blue fade rounds its green channel up",
      before:
        ".BlackLBlueFade:\n; Fade between black and light blue.\nfor hue, 32\n\tRGB 0, hue / 2, hue\nendr",
      after:
        ".BlackLBlueFade:\n; Fade between black and light blue.\nfor hue, 32\n\tRGB 0, (hue + 1) / 2, hue\nendr",
      diagnostic: /CrystalIntro_UnownFade.*BlackLBlueFade|generated RGB555 table/i,
    },
    {
      name: "the Unown fade redirects its palette request",
      before:
        "\tpop af\n\tldh [rWBK], a\n\tld a, TRUE\n\tldh [hCGBPalUpdate], a\n\tret\n\n.BWFade:",
      after:
        "\tpop af\n\tldh [rWBK], a\n\tld a, TRUE\n\tldh [hBGMapUpdate], a\n\tret\n\n.BWFade:",
      diagnostic: /CrystalIntro_UnownFade.*transfer request|hCGBPalUpdate/i,
    },
    {
      name: "the LY override reset clears a fixed shorter range",
      before:
        "\tld hl, wLYOverrides\n\tld bc, wLYOverridesEnd - wLYOverrides\n\txor a\n\tcall ByteFill",
      after:
        "\tld hl, wLYOverrides\n\tld bc, $8f\n\txor a\n\tcall ByteFill",
      diagnostic: /Intro_ResetLYOverrides.*banked clear|wLYOverridesEnd/i,
    },
    {
      name: "the perspective trees band updates every frame",
      before:
        "\tld a, [wIntroSceneFrameCounter]\n\tand $1\n\tjr z, .skip",
      after:
        "\tld a, [wIntroSceneFrameCounter]\n\tand $0\n\tjr z, .skip",
      diagnostic: /Intro_PerspectiveScrollBG.*trees band|and \$1/i,
    },
    {
      name: "the perspective grass band advances by one pixel",
      before:
        ".skip\n\t; grass in the front\n\tld hl, wLYOverrides + $5f\n\tld a, [hl]\n\tinc a\n\tinc a\n\tld bc, $31",
      after:
        ".skip\n\t; grass in the front\n\tld hl, wLYOverrides + $5f\n\tld a, [hl]\n\tinc a\n\tld bc, $31",
      diagnostic: /Intro_PerspectiveScrollBG.*grass band|inc a/i,
    },
    {
      name: "Scene 6 starts its second Unown early",
      before:
        "\tcp $60\n\tjr z, .SecondUnown\n\tcp $40\n\tjr nc, .StopUnown",
      after:
        "\tcp $58\n\tjr z, .SecondUnown\n\tcp $40\n\tjr nc, .StopUnown",
      diagnostic: /IntroScene6.*counter dispatch|cp \$60/i,
    },
    {
      name: "Scene 6 swaps the first Unown sound",
      before:
        ".FirstUnown:\n\tpush af\n\tdepixel 7, 15\n\tcall CrystalIntro_InitUnownAnim\n\tld de, SFX_INTRO_UNOWN_2",
      after:
        ".FirstUnown:\n\tpush af\n\tdepixel 7, 15\n\tcall CrystalIntro_InitUnownAnim\n\tld de, SFX_INTRO_UNOWN_1",
      diagnostic: /FirstUnown@IntroScene6.*trigger|SFX_INTRO_UNOWN_2/i,
    },
    {
      name: "Scene 7 shifts Suicune one pixel right",
      before:
        "\tdepixel 13, 27, 4, 0\n\tld a, SPRITE_ANIM_OBJ_INTRO_SUICUNE\n\tcall InitSpriteAnimStruct",
      after:
        "\tdepixel 13, 27, 5, 0\n\tld a, SPRITE_ANIM_OBJ_INTRO_SUICUNE\n\tcall InitSpriteAnimStruct",
      diagnostic: /IntroScene7.*allocation|depixel 13, 27, 4, 0/i,
    },
    {
      name: "Scene 8 moves Suicune four pixels per frame",
      before:
        ".animate_suicune\n\tld a, [wGlobalAnimXOffset]\n\tand a\n\tjr z, .finish\n\tsub $8",
      after:
        ".animate_suicune\n\tld a, [wGlobalAnimXOffset]\n\tand a\n\tjr z, .finish\n\tsub $4",
      diagnostic: /IntroScene8.*offset animation|sub \$8/i,
    },
    {
      name: "Scene 9 shortens its first attrmap band",
      before:
        "\thlcoord 0, 0, wAttrmap\n\t; first 12 rows have palette 1\n\tld bc, 12 * SCREEN_WIDTH",
      after:
        "\thlcoord 0, 0, wAttrmap\n\t; first 12 rows have palette 1\n\tld bc, 11 * SCREEN_WIDTH",
      diagnostic: /IntroScene9.*attrmap\/update program|12 \* SCREEN_WIDTH/i,
    },
    {
      name: "Scene 9 points at BG map byte $0d",
      before:
        "\tcall DelayFrame\n\tld a, LOW(vBGMap0 + $c) ; $c\n\tldh [hBGMapAddress], a",
      after:
        "\tcall DelayFrame\n\tld a, LOW(vBGMap0 + $d) ; $d\n\tldh [hBGMapAddress], a",
      diagnostic: /IntroScene9.*attrmap\/update program|LOW\(vBGMap0 \+ \$c\)/i,
    },
    {
      name: "the grass rustle lasts one extra frame",
      before:
        "Intro_RustleGrass:\n\tld a, [wIntroSceneFrameCounter]\n\tcp 36\n\tret nc",
      after:
        "Intro_RustleGrass:\n\tld a, [wIntroSceneFrameCounter]\n\tcp 37\n\tret nc",
      diagnostic: /Intro_RustleGrass.*gated asynchronous|cp 36/i,
    },
    {
      name: "the grass animation skips its repeated second frame",
      before:
        ".RustlingGrassPointers:\n\tdw IntroGrass1GFX\n\tdw IntroGrass2GFX\n\tdw IntroGrass3GFX\n\tdw IntroGrass2GFX",
      after:
        ".RustlingGrassPointers:\n\tdw IntroGrass1GFX\n\tdw IntroGrass2GFX\n\tdw IntroGrass3GFX\n\tdw IntroGrass1GFX",
      diagnostic: /Intro_RustleGrass.*four-entry animation table|IntroGrass2GFX/i,
    },
    {
      name: "Scene 12 changes its $90 Unown sound",
      before:
        "\tdbw $80, SFX_INTRO_UNOWN_3\n\tdbw $90, SFX_INTRO_UNOWN_2\n\tdbw $a0, SFX_INTRO_UNOWN_1",
      after:
        "\tdbw $80, SFX_INTRO_UNOWN_3\n\tdbw $90, SFX_INTRO_UNOWN_3\n\tdbw $a0, SFX_INTRO_UNOWN_1",
      diagnostic: /IntroScene12.*frame-to-sound table|SFX_INTRO_UNOWN_2/i,
    },
    {
      name: "Scene 12 changes its double-speed timer mask",
      before:
        ".second_half\n; double speed\n\tld c, a\n\tand $f\n\tsla a\n\tsla a",
      after:
        ".second_half\n; double speed\n\tld c, a\n\tand $1f\n\tsla a\n\tsla a",
      diagnostic: /IntroScene12.*double-speed second half|and \$f/i,
    },
  ])("fails closed when $name", ({ before, after, diagnostic }) => {
    const mutatedIntro = replaceExact(canonicalRead("engine/movie/intro.asm"), before, after);
    expect(() =>
      analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === "engine/movie/intro.asm"
            ? mutatedIntro
            : canonicalRead(relativePath),
      }),
    ).toThrow(diagnostic);
  });

  it("fails closed when the contiguous decompression overflow allocation shrinks", () => {
    const file = "ram/wram.asm";
    const mutatedWram = replaceExact(
      canonicalRead(file),
      "wDecompressScratch:: ds $80 tiles\nwDecompressEnemyFrontpic:: ds $80 tiles",
      "wDecompressScratch:: ds $80 tiles\nwDecompressEnemyFrontpic:: ds $7f tiles",
    );
    expect(() =>
      analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === file ? mutatedWram : canonicalRead(relativePath),
      }),
    ).toThrow(/decompression contiguous overflow capacity|wDecompressEnemyFrontpic/i);
  });

  it.each([
    {
      name: "the central scheduler redirects its OAM cursor",
      before:
        "\tld a, LOW(wShadowOAM)\n\tld [wCurSpriteOAMAddr], a\n\tcall DoNextFrameForAllSprites",
      after:
        "\tld a, LOW(wShadowOAM)\n\tld [wCurSpriteOAMFlags], a\n\tcall DoNextFrameForAllSprites",
      diagnostic: /PlaySpriteAnimations|OAM cursor|wCurSpriteOAMAddr/i,
    },
    {
      name: "the zero-instance scheduler stops clearing remaining OAM",
      before:
        "\tcp LOW(wShadowOAMEnd)\n\tjr nc, .done\n\txor a\n\tld [hli], a\n\tjr .loop2",
      after:
        "\tcp LOW(wShadowOAMEnd)\n\tjr nc, .done\n\tinc a\n\tld [hli], a\n\tjr .loop2",
      diagnostic: /PlaySpriteAnimations|remaining OAM|xor a/i,
    },
  ])("fails closed when $name", ({ before, after, diagnostic }) => {
    const file = "engine/sprite_anims/core.asm";
    const mutatedSource = replaceExact(canonicalRead(file), before, after);
    expect(() =>
      analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === file ? mutatedSource : canonicalRead(relativePath),
      }),
    ).toThrow(diagnostic);
  });

  it.each([
    {
      name: "the palette request helper uses a far call",
      file: "engine/movie/intro.asm",
      before: "\tcall Intro_SetCGBPalUpdate\n\txor a",
      after: "\tfarcall Intro_SetCGBPalUpdate\n\txor a",
      diagnostic: /Intro_SetCGBPalUpdate.*exact call|call form/i,
      replaceFirst: true,
    },
    {
      name: "the palette request helper contains an extra CPU side effect",
      file: "engine/movie/intro.asm",
      before:
        "Intro_SetCGBPalUpdate:\n\tld a, TRUE\n\tldh [hCGBPalUpdate], a\n\tret",
      after:
        "Intro_SetCGBPalUpdate:\n\tld a, TRUE\n\tldh [hCGBPalUpdate], a\n\tinc a\n\tret",
      diagnostic: /Intro_SetCGBPalUpdate.*exact|unsupported|inc a/i,
    },
    {
      name: "the animation reset uses the opposite far-call macro form",
      file: "engine/movie/intro.asm",
      before: "\tfarcall ClearSpriteAnims\n\tcall Intro_SetCGBPalUpdate",
      after: "\tcallfar ClearSpriteAnims\n\tcall Intro_SetCGBPalUpdate",
      diagnostic: /ClearSpriteAnims.*exact farcall|farcall.*ClearSpriteAnims/i,
      replaceFirst: true,
    },
    {
      name: "the mobile alias is shorter than the sprite-animation view",
      file: "ram/wram.asm",
      before: "wc3ac:: ds 8\nENDU\n\nwSpriteAnimCount:: db",
      after: "wc3ac:: ds 7\nENDU\n\nwSpriteAnimCount:: db",
      diagnostic: /wSpriteAnimData|mobile.*alias|180/i,
    },
    {
      name: "the far-call return restores the ROM bank from the flags byte",
      file: "home/farcall.asm",
      before: "\tpop bc\n\tld a, b\n\trst Bankswitch",
      after: "\tpop bc\n\tld a, c\n\trst Bankswitch",
      diagnostic: /ClearSpriteAnims|farcall|ROM bank|ReturnFarCall/i,
    },
    {
      name: "CopyBytes decrements rather than increments its destination",
      file: "home/copy.asm",
      before: "\tld [de], a\n\tinc de\n.HandleLoop:",
      after: "\tld [de], a\n\tdec de\n.HandleLoop:",
      diagnostic: /CopyBytes|ascending|inc de/i,
    },
    {
      name: "the included palette contains a non-RGB data directive",
      file: "gfx/intro/unowns.pal",
      before: "\tRGB 31,  0, 31",
      after: "\tdb $ff, $7f",
      diagnostic: /IntroUnownsPalette|unowns\.pal|RGB|palette/i,
    },
    {
      name: "a decompression helper passes the wrong source bank",
      file: "engine/movie/intro.asm",
      before: "\tlb bc, $01, $40\n\tcall Request2bpp",
      after: "\tlb bc, $02, $40\n\tcall Request2bpp",
      diagnostic:
        /Intro_DecompressRequest2bpp_64Tiles|source-derived|\$01 bank/i,
    },
    {
      name: "a compressed resource label no longer owns an INCBIN payload",
      file: "engine/movie/intro.asm",
      before: 'IntroPulseGFX:\nINCBIN "gfx/intro/pulse.2bpp.lz"',
      after: "IntroPulseGFX:\ndb $ff",
      diagnostic: /IntroPulseGFX|INCBIN|resource/i,
    },
    {
      name: "the VBlank request service runs after the 1bpp request",
      file: "home/vblank.asm",
      before:
        "\tcall Serve2bppRequest\n\tcall Serve1bppRequest\n\tcall AnimateTileset",
      after:
        "\tcall Serve1bppRequest\n\tcall Serve2bppRequest\n\tcall AnimateTileset",
      diagnostic: /Serve2bppRequest|VBlank|priority|order/i,
    },
  ])(
    "fails closed when $name",
    ({ file, before, after, diagnostic, replaceFirst = false }) => {
      const mutated = replaceFirst
        ? replaceFirstExact(canonicalRead(file), before, after)
        : replaceExact(canonicalRead(file), before, after);
      expect(() =>
        analyzeRuntimeTitlePresentationEmission({
          disassemblyRoot,
          audioAssetIds: new Set(),
          runtimeSpawnIdentifiers: new Set([0]),
          readSource: (relativePath) =>
            relativePath === file ? mutated : canonicalRead(relativePath),
        }),
      ).toThrow(diagnostic);
    },
  );

  it("derives the palette-request target from the helper source", () => {
    const file = "engine/movie/intro.asm";
    const mutated = replaceExact(
      canonicalRead(file),
      "Intro_SetCGBPalUpdate:\n\tld a, TRUE\n\tldh [hCGBPalUpdate], a\n\tret",
      "Intro_SetCGBPalUpdate:\n\tld a, TRUE\n\tldh [hBGMapMode], a\n\tret",
    );
    const checkpoint = analyzeRuntimeTitlePresentationEmission({
      disassemblyRoot,
      audioAssetIds: new Set(),
      runtimeSpawnIdentifiers: new Set([0]),
      readSource: (relativePath) =>
        relativePath === file ? mutated : canonicalRead(relativePath),
    });
    const helperWrite = crystalIntroOperations(checkpoint).find(
      (operation) =>
        operation.op === "write_memory_byte" &&
        (operation.invocation as { target?: string } | undefined)?.target ===
          "Intro_SetCGBPalUpdate",
    );
    expect(helperWrite).toMatchObject({
      target: "hBGMapMode",
      value: 1,
      address_space: "hram",
    });
  });

  it.each([
    {
      name: "the 64-tile helper queues only 63 tiles",
      before: "\tlb bc, $01, $40\n\tcall Request2bpp",
      after: "\tlb bc, $01, $3f\n\tcall Request2bpp",
      operationIndex: 1,
      expected: {
        op: "request_2bpp_transfer",
        tile_count: 63,
        byte_count: 1008,
      },
    },
    {
      name: "the 128-tile helper queues only 127 tiles",
      before: "\tlb bc, $01, $80\n\tcall Request2bpp",
      after: "\tlb bc, $01, $7f\n\tcall Request2bpp",
      operationIndex: 3,
      expected: {
        op: "request_2bpp_transfer",
        tile_count: 127,
        byte_count: 2032,
      },
    },
    {
      name: "the attribute-map operand points at the tilemap resource",
      before:
        "\tld hl, IntroUnownAAttrmap\n\tdebgcoord 0, 0 ; vBGMap2\n\tcall Intro_DecompressRequest2bpp_64Tiles",
      after:
        "\tld hl, IntroUnownATilemap\n\tdebgcoord 0, 0 ; vBGMap2\n\tcall Intro_DecompressRequest2bpp_64Tiles",
      operationIndex: 0,
      expected: {
        op: "decompress_lz3_resource",
        resource_symbol: "IntroUnownATilemap",
        resource: "gfx/intro/unown_a.tilemap.lz",
      },
    },
    {
      name: "the attribute-map destination moves one tile right",
      before:
        "\tld hl, IntroUnownAAttrmap\n\tdebgcoord 0, 0 ; vBGMap2\n\tcall Intro_DecompressRequest2bpp_64Tiles",
      after:
        "\tld hl, IntroUnownAAttrmap\n\tdebgcoord 1, 0 ; vBGMap2\n\tcall Intro_DecompressRequest2bpp_64Tiles",
      operationIndex: 1,
      expected: {
        op: "request_2bpp_transfer",
        target: "vBGMap0 + 1",
        target_byte_offset: 1,
      },
    },
    {
      name: "the pulse label names the Unown payload",
      before: 'IntroPulseGFX:\nINCBIN "gfx/intro/pulse.2bpp.lz"',
      after: 'IntroPulseGFX:\nINCBIN "gfx/intro/unowns.2bpp.lz"',
      operationIndex: 4,
      expected: {
        op: "decompress_lz3_resource",
        resource_symbol: "IntroPulseGFX",
        resource: "gfx/intro/unowns.2bpp.lz",
        output_byte_count: 2048,
      },
    },
  ])(
    "derives changed semantics when $name",
    ({ before, after, operationIndex, expected }) => {
      const file = "engine/movie/intro.asm";
      const mutated = replaceExact(canonicalRead(file), before, after);
      const checkpoint = analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === file ? mutated : canonicalRead(relativePath),
      });
      const transferOperations =
        crystalIntroOperations(checkpoint).filter((operation) =>
          ["decompress_lz3_resource", "request_2bpp_transfer"].includes(
            operation.op,
          ),
        ) ?? [];
      expect(transferOperations[operationIndex]).toMatchObject(expected);
    },
  );

  it.each([
    {
      name: "the first palette copy is shortened to fifteen palettes",
      before:
        "\tld hl, IntroUnownsPalette\n\tld de, wBGPals1\n\tld bc, 16 palettes\n\tcall CopyBytes",
      after:
        "\tld hl, IntroUnownsPalette\n\tld de, wBGPals1\n\tld bc, 15 palettes\n\tcall CopyBytes",
      operationIndex: 0,
      expected: {
        op: "copy_memory",
        target: "wBGPals1",
        byte_count: 120,
        destination_segments: [
          { target: "wBGPals1", target_offset: 0, byte_count: 64 },
          { target: "wOBPals1", target_offset: 0, byte_count: 56 },
        ],
      },
    },
    {
      name: "the first palette copy begins at the object palettes",
      before:
        "\tld hl, IntroUnownsPalette\n\tld de, wBGPals1\n\tld bc, 16 palettes\n\tcall CopyBytes",
      after:
        "\tld hl, IntroUnownsPalette\n\tld de, wOBPals1\n\tld bc, 16 palettes\n\tcall CopyBytes",
      operationIndex: 0,
      expected: {
        op: "copy_memory",
        target: "wOBPals1",
        byte_count: 128,
        destination_segments: [
          { target: "wOBPals1", target_offset: 0, byte_count: 64 },
          { target: "wBGPals2", target_offset: 0, byte_count: 64 },
        ],
      },
    },
  ])(
    "derives changed copy semantics when $name",
    ({ before, after, operationIndex, expected }) => {
      const file = "engine/movie/intro.asm";
      const mutated = replaceFirstExact(canonicalRead(file), before, after);
      const checkpoint = analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === file ? mutated : canonicalRead(relativePath),
      });
      const copyOperations =
        crystalIntroOperations(checkpoint).filter(
          (operation) => operation.op === "copy_memory",
        ) ?? [];
      expect(copyOperations[operationIndex]).toMatchObject(expected);
    },
  );

  it.each([
    {
      name: "the intro palette clear covers only fifteen palettes",
      before:
        "\tld hl, wBGPals2\n\tld bc, 16 palettes\n\txor a\n\tcall ByteFill",
      after:
        "\tld hl, wBGPals2\n\tld bc, 15 palettes\n\txor a\n\tcall ByteFill",
      diagnostic: /Intro_ClearBGPals|16 palettes|128/i,
    },
    {
      name: "the intro palette request is redirected",
      before: "\tld a, TRUE\n\tldh [hCGBPalUpdate], a\n\tcall DelayFrame",
      after: "\tld a, TRUE\n\tldh [hBGMapUpdate], a\n\tcall DelayFrame",
      diagnostic: /Intro_ClearBGPals|hCGBPalUpdate|request/i,
    },
    {
      name: "the second post-request VBlank is removed",
      before: "\tcall DelayFrame\n\tcall DelayFrame\n\tret",
      after: "\tcall DelayFrame\n\tret",
      diagnostic: /Intro_ClearBGPals|two|VBlank|DelayFrame/i,
    },
  ])("fails closed when $name", ({ before, after, diagnostic }) => {
    const mutatedIntro = replaceExact(
      canonicalRead("engine/movie/intro.asm"),
      before,
      after,
    );
    expect(() =>
      analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === "engine/movie/intro.asm"
            ? mutatedIntro
            : canonicalRead(relativePath),
      }),
    ).toThrow(diagnostic);
  });

  it("fails closed when CrystalIntro does not restore the saved menu state", () => {
    const mutatedIntro = replaceExact(
      canonicalRead("engine/movie/intro.asm"),
      "\tpop af\n\tldh [hInMenu], a\n\tpop af\n\tldh [rWBK], a",
      "\tpop af\n\tldh [hMapAnims], a\n\tpop af\n\tldh [rWBK], a",
    );
    expect(() =>
      analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === "engine/movie/intro.asm"
            ? mutatedIntro
            : canonicalRead(relativePath),
      }),
    ).toThrow(/CrystalIntro|hInMenu|restore/i);
  });

  it.each([
    {
      name: "the central sprite scheduler moves after the VBlank wait",
      before:
        "\tcall IntroSceneJumper\n\tfarcall PlaySpriteAnimations\n\tcall DelayFrame",
      after:
        "\tcall IntroSceneJumper\n\tcall DelayFrame\n\tfarcall PlaySpriteAnimations",
      diagnostic: /CrystalIntro|sprite|VBlank|order/i,
    },
    {
      name: "the first scene-table entry no longer names IntroScene1",
      before: "IntroScenes:\n\tdw IntroScene1\n\tdw IntroScene2",
      after: "IntroScenes:\n\tdw IntroScene2\n\tdw IntroScene2",
      diagnostic:
        /CrystalIntro|scene dispatcher|sprite closure IntroScenes|dispatcher entry 0|IntroScene1/i,
    },
  ])("fails closed when $name", ({ before, after, diagnostic }) => {
    const mutatedIntro = replaceExact(
      canonicalRead("engine/movie/intro.asm"),
      before,
      after,
    );
    expect(() =>
      analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === "engine/movie/intro.asm"
            ? mutatedIntro
            : canonicalRead(relativePath),
      }),
    ).toThrow(diagnostic);
  });

  it("does not infer a carry-result branch from another CPU flag", () => {
    const mutatedIntro = replaceExact(
      canonicalRead(introPath),
      "\tcallfar SplashScreen\n\tjr c, StartTitleScreen",
      "\tcallfar SplashScreen\n\tjr z, StartTitleScreen",
    );
    const checkpoint = analyzeRuntimeTitlePresentationEmission({
      disassemblyRoot,
      audioAssetIds: new Set(),
      runtimeSpawnIdentifiers: new Set([0]),
      readSource: (relativePath) =>
        relativePath === introPath ? mutatedIntro : canonicalRead(relativePath),
    });
    expect(checkpoint.frontier).toEqual({
      reason: "missing_runtime_operation",
      block: "IntroSequence",
      target: "StartTitleScreen",
      opcode: "jr",
      args: ["z", "StartTitleScreen"],
      source_span: {
        file: introPath,
        start_line: 966,
        end_line: 966,
      },
    });
  });

  it.each([
    {
      name: "the VRAM high-byte source symbol is not declared",
      file: vramPath,
      before: "vBGMap0:: ds TILEMAP_AREA",
      after: "vOtherBGMap0:: ds TILEMAP_AREA",
      diagnostic: /HIGH\(vBGMap0\)|source symbol|ram\/vram\.asm/i,
    },
    {
      name: "an ldh target is outside high memory",
      file: introPath,
      before: "\tldh [hJoyDown], a",
      after: "\tldh [wJoyDown], a",
      diagnostic: /ldh|wJoyDown|high.memory/i,
    },
  ])("fails closed when $name", ({ file, before, after, diagnostic }) => {
    const mutated = replaceExact(canonicalRead(file), before, after);
    expect(() =>
      analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === file ? mutated : canonicalRead(relativePath),
      }),
    ).toThrow(diagnostic);
  });

  it("requires exact source evidence for symbolic byte writes", () => {
    const sourceSpan = { file: introPath, start_line: 1334, end_line: 1335 };
    const program = {
      schema_version: 1,
      entrypoints: Object.fromEntries(
        RUNTIME_PRESENTATION_ENTRYPOINTS.map((entrypoint) => [
          entrypoint,
          "Boot",
        ]),
      ),
      blocks: {
        Boot: {
          source_span: sourceSpan,
          operations: [
            {
              op: "write_memory_byte",
              target: "hBGMapAddress + 1",
              value: "HIGH(vBGMap0)",
              address_space: "hram",
              condition: {
                source: null,
                predicate: "always",
                source_span: null,
              },
              value_source_span: {
                file: vramPath,
                start_line: 6,
                end_line: 6,
              },
              source_span: sourceSpan,
            },
          ],
        },
      },
      resources: [],
      audio: [],
      text: [],
      host_effects: [],
      subprograms: [],
    };
    expect(() => assertRuntimePresentationProgram(program)).not.toThrow();

    const missingEvidence = structuredClone(program);
    delete (
      missingEvidence.blocks.Boot.operations[0] as Record<string, unknown>
    ).value_source_span;
    expect(() => assertRuntimePresentationProgram(missingEvidence)).toThrow(
      /write_memory_byte|source/i,
    );

    const unsupportedExpression = structuredClone(program);
    unsupportedExpression.blocks.Boot.operations[0].value = "HIGH(vBGMap0) + 1";
    expect(() =>
      assertRuntimePresentationProgram(unsupportedExpression),
    ).toThrow(/write_memory_byte/i);
  });

  it.each([
    {
      name: "the cancellation return no longer sets carry",
      file: splashPath,
      before: ".pressed_button\n\tcall GameFreakPresentsEnd\n\tscf\n\tret",
      after: ".pressed_button\n\tcall GameFreakPresentsEnd\n\tand a\n\tret",
      diagnostic: /SplashScreen|cancel|carry|scf/i,
    },
    {
      name: "the natural return no longer clears carry",
      file: splashPath,
      before: ".finish\n\tcall GameFreakPresentsEnd\n\tand a\n\tret",
      after: ".finish\n\tcall GameFreakPresentsEnd\n\tscf\n\tret",
      diagnostic: /SplashScreen|complete|carry|and a/i,
    },
    {
      name: "the DMG path is no longer the non-returning branch",
      file: gbcOnlyPath,
      before: "\tldh a, [hCGB]\n\tand a\n\tret nz",
      after: "\tldh a, [hCGB]\n\tand a\n\tret z",
      diagnostic: /GBCOnlyScreen|hCGB|non.return|ret nz/i,
    },
    {
      name: "the splash loop bypasses JoyTextDelay",
      file: splashPath,
      before: ".joy_loop\n\tcall JoyTextDelay\n\tldh a, [hJoyLast]",
      after: ".joy_loop\n\tcall GetJoypad\n\tldh a, [hJoyLast]",
      diagnostic: /SplashScreen|JoyTextDelay|input/i,
    },
    {
      name: "the initial fixed wait changes",
      file: splashPath,
      before: "\tld c, 10\n\tcall DelayFrames\n\n; Draw copyright screen",
      after: "\tld c, 11\n\tcall DelayFrames\n\n; Draw copyright screen",
      diagnostic: /SplashScreen|10|fixed wait/i,
    },
    {
      name: "the teardown fixed wait changes",
      file: splashPath,
      before: "\tcall ClearSprites\n\tld c, 16\n\tcall DelayFrames\n\tret",
      after: "\tcall ClearSprites\n\tld c, 15\n\tcall DelayFrames\n\tret",
      diagnostic: /GameFreakPresentsEnd|16|teardown/i,
    },
    {
      name: "the copyright graphics resource changes",
      file: miscGraphicsPath,
      before: 'INCBIN "gfx/splash/copyright.2bpp"',
      after: 'INCBIN "gfx/splash/copyright-mutated.2bpp"',
      diagnostic: /CopyrightGFX|copyright\.2bpp|resource/i,
    },
    {
      name: "LoadFontsExtra copies one fewer source-derived glyph",
      file: loadFontPath,
      before:
        "\tld hl, vTiles2 tile '<BOLD_D>'\n\tlb bc, BANK(FontExtra), 22 ; '<BOLD_D>' to 'ぉ'",
      after:
        "\tld hl, vTiles2 tile '<BOLD_D>'\n\tlb bc, BANK(FontExtra), 21 ; '<BOLD_D>' to 'ぉ'",
      diagnostic: /LoadFontsExtra|FontExtra|22/i,
    },
    {
      name: "the SGB layout table no longer selects the Game Freak palette packet",
      file: sgbLayoutsPath,
      before: "\tdw .SGB_GamefreakLogo\n\tdw .SGB_PlayerOrMonFrontpicPals",
      after:
        "\tdw .SGB_GamefreakLogoMutated\n\tdw .SGB_PlayerOrMonFrontpicPals",
      diagnostic: /SGB.*GamefreakLogo|SCGB_GAMEFREAK_LOGO|layout/i,
    },
    {
      name: "the CGB layout no longer writes the second object palette",
      file: cgbLayoutsPath,
      before:
        "\tld hl, .GamefreakDittoPalette\n\tld de, wOBPals1 palette 1\n\tcall LoadHLPaletteIntoDE",
      after:
        "\tld hl, .GamefreakDittoPalette\n\tld de, wOBPals1 palette 2\n\tcall LoadHLPaletteIntoDE",
      diagnostic: /CGB.*GamefreakLogo|wOBPals1 palette 1|layout/i,
    },
    {
      name: "the source CGB Game Freak background palette changes",
      file: predefPalettesPath,
      before:
        "\tRGB 00,00,00, 08,11,11, 21,21,21, 31,31,31 ; PREDEFPAL_GAMEFREAK_LOGO_BG",
      after:
        "\tRGB 00,00,00, 08,11,11, 21,21,20, 31,31,31 ; PREDEFPAL_GAMEFREAK_LOGO_BG",
      diagnostic: /CGB Game Freak predefined background palette|palette data/i,
    },
  ])(
    "fails closed for the callable splash contract when $name",
    ({ file, before, after, diagnostic }) => {
      const mutated = replaceExact(canonicalRead(file), before, after);
      expect(() =>
        analyzeRuntimeTitlePresentationEmission({
          disassemblyRoot,
          audioAssetIds: new Set(),
          runtimeSpawnIdentifiers: new Set([0]),
          readSource: (relativePath) =>
            relativePath === file ? mutated : canonicalRead(relativePath),
        }),
      ).toThrow(diagnostic);
    },
  );

  it.each([
    {
      name: "the tilemap fill is not the space glyph",
      file: textPath,
      before: "\thlcoord 0, 0\n\tld a, ' '\n\tld bc, wTilemapEnd - wTilemap",
      after: "\thlcoord 0, 0\n\txor a\n\tld bc, wTilemapEnd - wTilemap",
      diagnostic: /ClearTilemap|space|ld a, ' '/i,
    },
    {
      name: "the tilemap fill starts at another coordinate",
      file: textPath,
      before: "\thlcoord 0, 0\n\tld a, ' '",
      after: "\thlcoord 1, 0\n\tld a, ' '",
      diagnostic: /ClearTilemap|hlcoord 0, 0|wTilemap/i,
    },
    {
      name: "the source tilemap is not exactly one screen",
      file: wramPath,
      before:
        "wTilemap::\n; 20x18 grid of 8x8 tiles\n\tds SCREEN_AREA\nwTilemapEnd::",
      after:
        "wTilemap::\n; 20x18 grid of 8x8 tiles\n\tds SCREEN_AREA - 1\nwTilemapEnd::",
      diagnostic: /ClearTilemap|360|tilemap.*region/i,
    },
    {
      name: "the space glyph source value changes",
      file: charmapPath,
      before: '\tcharmap " ",         $7f',
      after: '\tcharmap " ",         $00',
      diagnostic: /space glyph|\$7f|charmap/i,
    },
    {
      name: "the LCD enable guard tests another bit",
      file: textPath,
      before: "\tbit B_LCDC_ENABLE, a\n\tret z\n\tjp WaitBGMap",
      after: "\tbit B_LCDC_WIN_ENABLE, a\n\tret z\n\tjp WaitBGMap",
      diagnostic: /ClearTilemap|B_LCDC_ENABLE|LCD/i,
    },
    {
      name: "the LCD-enabled path no longer schedules WaitBGMap",
      file: textPath,
      before: "\tret z\n\tjp WaitBGMap\n\nClearScreen::",
      after: "\tret z\n\tret\n\nClearScreen::",
      diagnostic: /ClearTilemap|WaitBGMap|schedule/i,
    },
    {
      name: "hlcoord no longer resolves the default origin to wTilemap",
      file: coordsMacrosPath,
      before:
        "\tif _NARG < 4\n\t\tld \\1, (\\3) * SCREEN_WIDTH + (\\2) + wTilemap",
      after:
        "\tif _NARG < 4\n\t\tld \\1, (\\3) * SCREEN_WIDTH + (\\2) + wAttrmap",
      diagnostic: /hlcoord|coord.*macro|wTilemap/i,
    },
  ])(
    "fails closed for ClearTilemap when $name",
    ({ file, before, after, diagnostic }) => {
      const mutated = replaceExact(canonicalRead(file), before, after);
      expect(() =>
        analyzeRuntimeTitlePresentationEmission({
          disassemblyRoot,
          audioAssetIds: new Set(),
          runtimeSpawnIdentifiers: new Set([0]),
          readSource: (relativePath) =>
            relativePath === file ? mutated : canonicalRead(relativePath),
        }),
      ).toThrow(diagnostic);
    },
  );

  it.each([
    {
      name: "the DMG object palettes are not both cleared",
      file: tilemapPath,
      before:
        "\txor a\n\tldh [rBGP], a\n\tldh [rOBP0], a\n\tldh [rOBP1], a\n\tret",
      after:
        "\txor a\n\tldh [rBGP], a\n\tldh [rOBP0], a\n\tldh [rOBP0], a\n\tret",
      diagnostic: /ClearPalettes|rOBP1/i,
    },
    {
      name: "the CGB white fill is not exactly sixteen palettes",
      file: tilemapPath,
      before: "\tld bc, 16 palettes\n\tld a, $ff",
      after: "\tld bc, 15 palettes\n\tld a, $ff",
      diagnostic: /ClearPalettes|16 palettes|128/i,
    },
    {
      name: "the CGB palette request flag is redirected",
      file: tilemapPath,
      before: "\tld a, TRUE\n\tldh [hCGBPalUpdate], a\n\tret",
      after: "\tld a, TRUE\n\tldh [hBGMapUpdate], a\n\tret",
      diagnostic: /ClearPalettes|hCGBPalUpdate|palette.*request/i,
    },
    {
      name: "WaitBGMap no longer waits four frames",
      file: tilemapPath,
      before:
        "WaitBGMap::\n; Tell VBlank to update BG Map\n\tld a, 1 ; BG Map 0 tiles\n\tldh [hBGMapMode], a\n; Wait for it to do its magic\n\tld c, 4\n\tcall DelayFrames\n\tret",
      after:
        "WaitBGMap::\n; Tell VBlank to update BG Map\n\tld a, 1 ; BG Map 0 tiles\n\tldh [hBGMapMode], a\n; Wait for it to do its magic\n\tld c, 3\n\tcall DelayFrames\n\tret",
      diagnostic: /WaitBGMap|four|ld c, 4/i,
    },
    {
      name: "the VBlank transfer no longer clears its request",
      file: palettesPath,
      before:
        "; clear pal update queue\n\txor a\n\tldh [hCGBPalUpdate], a\n\n\tscf",
      after:
        "; clear pal update queue\n\txor a\n\tldh [hBGMapUpdate], a\n\n\tscf",
      diagnostic: /UpdateCGBPals|hCGBPalUpdate|completion/i,
    },
    {
      name: "the second palette buffer is not eight palettes",
      file: wramPath,
      before: "wBGPals2:: ds 8 palettes\nwOBPals2:: ds 8 palettes",
      after: "wBGPals2:: ds 7 palettes\nwOBPals2:: ds 8 palettes",
      diagnostic: /palette.*buffer|128|wBGPals2/i,
    },
  ])(
    "fails closed for ClearBGPalettes when $name",
    ({ file, before, after, diagnostic }) => {
      const mutated = replaceExact(canonicalRead(file), before, after);
      expect(() =>
        analyzeRuntimeTitlePresentationEmission({
          disassemblyRoot,
          audioAssetIds: new Set(),
          runtimeSpawnIdentifiers: new Set([0]),
          readSource: (relativePath) =>
            relativePath === file ? mutated : canonicalRead(relativePath),
        }),
      ).toThrow(diagnostic);
    },
  );

  it.each([
    {
      name: "one of the four menu regions is no longer cleared",
      file: homeMenuPath,
      before:
        "\tld hl, wMoreMenuData\n\tcall .ClearMenuData\n\n\tldh a, [rWBK]",
      after:
        "\tld hl, wMoreMenuDataEnd\n\tcall .ClearMenuData\n\n\tldh a, [rWBK]",
      diagnostic: /ClearWindowData|wMoreMenuData/i,
    },
    {
      name: "the shared menu clear no longer invokes ByteFill",
      file: homeMenuPath,
      before: "\txor a\n\tcall ByteFill\n\tret",
      after: "\txor a\n\tcall CopyBytes\n\tret",
      diagnostic: /ClearWindowData|ClearMenuData|ByteFill/i,
    },
    {
      name: "ByteFill no longer writes every byte",
      file: copyPath,
      before: ".PutByte:\n\tld [hli], a\n.HandleLoop:\n\tdec c",
      after: ".PutByte:\n\tinc hl\n.HandleLoop:\n\tdec c",
      diagnostic: /ByteFill|ld \[hli\], a/i,
    },
    {
      name: "the four menu regions are no longer exactly equal-sized",
      file: wramPath,
      before: "wWindowStackSize:: db\n\tds 8\nwMenuMetadataEnd::",
      after: "wWindowStackSize:: db\n\tds 7\nwMenuMetadataEnd::",
      diagnostic: /menu.*region|16|wMenuMetadata/i,
    },
    {
      name: "the window stack bank is not restored",
      file: homeMenuPath,
      before: "\tpop af\n\tldh [rWBK], a\n\tret",
      after: "\tpop af\n\tldh [hTempBank], a\n\tret",
      diagnostic: /ClearWindowData|rWBK|restore/i,
    },
    {
      name: "the stack pointer no longer follows two descending writes",
      file: homeMenuPath,
      before: "\tld [hld], a\n\tld [hld], a\n\tld a, l",
      after: "\tld [hli], a\n\tld [hli], a\n\tld a, l",
      diagnostic: /ClearWindowData|hld|stack pointer/i,
    },
  ])("fails closed when $name", ({ file, before, after, diagnostic }) => {
    const mutated = replaceExact(canonicalRead(file), before, after);
    expect(() =>
      analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === file ? mutated : canonicalRead(relativePath),
      }),
    ).toThrow(diagnostic);
  });

  it.each([
    {
      name: "the primary boot slice is not exactly eight bytes",
      file: savePath,
      before:
        "\tld de, wStartDay\n\tld bc, 8\n\tcall CopyBytes\n\tld hl, sPlayerData + wStatusFlags - wPlayerData",
      after:
        "\tld de, wStartDay\n\tld bc, 7\n\tcall CopyBytes\n\tld hl, sPlayerData + wStatusFlags - wPlayerData",
      diagnostic: /TryLoadSaveData|boot slice|ld bc, 8/i,
    },
    {
      name: "the backup discriminator is no longer two",
      file: savePath,
      before: "\tld a, $2\n\tld [wSaveFileExists], a",
      after: "\tld a, $3\n\tld [wSaveFileExists], a",
      diagnostic: /CheckBackupSaveFile|discriminator|\$2/i,
    },
    {
      name: "the status byte is copied to another destination",
      file: savePath,
      before:
        "\tld hl, sPlayerData + wStatusFlags - wPlayerData\n\tld de, wStatusFlags\n\tld a, [hl]\n\tld [de], a\n\tcall CloseSRAM",
      after:
        "\tld hl, sPlayerData + wStatusFlags - wPlayerData\n\tld de, wStatusFlags2\n\tld a, [hl]\n\tld [de], a\n\tcall CloseSRAM",
      diagnostic: /TryLoadSaveData|wStatusFlags/i,
    },
    {
      name: "the default options no longer cover the exact option slice",
      file: defaultOptionsPath,
      before:
        "\tdb $00\n\tdb $00\n.End\n\tassert DefaultOptions.End - DefaultOptions == wOptionsEnd - wOptions",
      after:
        "\tdb $00\n.End\n\tassert DefaultOptions.End - DefaultOptions == wOptionsEnd - wOptions",
      diagnostic: /DefaultOptions|eight|option slice/i,
    },
    {
      name: "ClearClock does not zero every RTC byte",
      file: timePath,
      before: "\tldh [hRTCDayHi], a\n\tret",
      after: "\tldh [hRTCDayLo], a\n\tret",
      diagnostic: /ClearClock|hRTCDayHi|RTC/i,
    },
    {
      name: "SetClock no longer persists the day-high byte",
      file: timePath,
      before:
        "\tld [hl], RAMB_RTC_DH\n\tldh a, [hRTCDayHi]\n\tres B_RAMB_RTC_DH_HALT, a ; make sure timer is active\n\tld [de], a",
      after:
        "\tld [hl], RAMB_RTC_DH\n\tldh a, [hRTCDayLo]\n\tres B_RAMB_RTC_DH_HALT, a ; make sure timer is active\n\tld [de], a",
      diagnostic: /SetClock|hRTCDayHi|persist/i,
    },
  ])("fails closed when $name", ({ file, before, after, diagnostic }) => {
    const mutated = replaceExact(canonicalRead(file), before, after);
    expect(() =>
      analyzeRuntimeTitlePresentationEmission({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
        readSource: (relativePath) =>
          relativePath === file ? mutated : canonicalRead(relativePath),
      }),
    ).toThrow(diagnostic);
  });

  it("derives all eight roots, exact local targets, spans, and input samples from ASM", () => {
    const controlFlow = analyzeRuntimePresentationControlFlow({
      disassemblyRoot,
    });

    expect(Object.keys(controlFlow.entrypoints).sort()).toEqual(
      [...RUNTIME_PRESENTATION_ENTRYPOINTS].sort(),
    );
    expect(controlFlow.entrypoints).toEqual({
      boot: "GameInit",
      intro: "IntroSequence",
      title: "StartTitleScreen",
      main_menu: "Intro_MainMenu",
      continue: "Continue",
      new_game: "NewGame",
      delete_save: "DeleteSaveData",
      reset_clock: "ResetClock",
    });
    expect(controlFlow.blocks.Continue.direct_targets).toEqual(
      expect.arrayContaining([
        ".FailToLoad@Continue",
        ".Check1Pass@Continue",
        "ConfirmContinue",
      ]),
    );
    expect(controlFlow.blocks[".Check1Pass@Continue"].direct_targets).toEqual(
      expect.arrayContaining([
        ".Check2Pass@Continue",
        "Continue_CheckRTC_RestartClock",
      ]),
    );
    expect(controlFlow.blocks[".FailToLoad@Continue"]).toBeDefined();
    expect(controlFlow.blocks[".Check2Pass@Continue"]).toBeDefined();

    const heldRead = controlFlow.blocks.TitleScreenMain.instructions.find(
      (instruction) =>
        instruction.opcode === "ld" &&
        instruction.args[0] === "hl" &&
        instruction.args[1] === "hJoyDown",
    );
    expect(heldRead).toMatchObject({
      source_span: {
        file: introPath,
        start_line: 1155,
        end_line: 1155,
      },
    });
    const titleInputInstructions = Object.entries(controlFlow.blocks)
      .filter(
        ([block]) =>
          block === "TitleScreenMain" || block.endsWith("@TitleScreenMain"),
      )
      .flatMap(([, block]) => block.instructions);
    expect(titleInputInstructions).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          opcode: "and",
          args: ["PAD_UP + PAD_B + PAD_SELECT"],
        }),
        expect.objectContaining({
          opcode: "and",
          args: ["PAD_DOWN + PAD_B + PAD_SELECT"],
        }),
        expect.objectContaining({
          opcode: "and",
          args: ["PAD_START | PAD_A"],
        }),
      ]),
    );

    const pressedRead = Object.entries(controlFlow.blocks)
      .filter(
        ([block]) =>
          block === "ConfirmContinue" || block.endsWith("@ConfirmContinue"),
      )
      .flatMap(([, block]) => block.instructions)
      .find(
        (instruction) =>
          instruction.opcode === "ld" &&
          instruction.args[0] === "hl" &&
          instruction.args[1] === "hJoyPressed",
      );
    expect(pressedRead).toMatchObject({
      source_span: {
        file: introPath,
        start_line: 433,
        end_line: 433,
      },
    });
    for (const block of Object.values(controlFlow.blocks)) {
      expect(block.source_span.start_line).toBeGreaterThan(0);
      expect(block.source_span.end_line).toBeGreaterThanOrEqual(
        block.source_span.start_line,
      );
      for (const instruction of block.instructions) {
        expect(instruction.source_span.start_line).toBeGreaterThanOrEqual(
          block.source_span.start_line,
        );
        expect(instruction.source_span.end_line).toBeLessThanOrEqual(
          block.source_span.end_line,
        );
      }
    }
  });

  it("resolves anonymous branches by exact relative instance and rejects missing instances", () => {
    const intro = canonicalRead(introPath);
    const withForwardLabel = intro.replace(
      "\tcall DelayFrame\n\tld de, MUSIC_MAIN_MENU",
      "\tjr +\n\tld de, MUSIC_TITLE\n+\n\tcall DelayFrame\n\tld de, MUSIC_MAIN_MENU",
    );
    const controlFlow = analyzeRuntimePresentationControlFlow({
      disassemblyRoot,
      readSource: (relativePath) =>
        relativePath === introPath
          ? withForwardLabel
          : canonicalRead(relativePath),
    });
    const anonymous = Object.keys(controlFlow.blocks).find((label) =>
      label.startsWith("+@Intro_MainMenu@"),
    );
    expect(anonymous).toBeDefined();
    expect(controlFlow.blocks.Intro_MainMenu.direct_targets).toContain(
      anonymous,
    );

    const withMissingSecondForwardLabel = withForwardLabel.replace(
      "\tjr +",
      "\tjr ++",
    );
    expect(() =>
      analyzeRuntimePresentationControlFlow({
        disassemblyRoot,
        readSource: (relativePath) =>
          relativePath === introPath
            ? withMissingSecondForwardLabel
            : canonicalRead(relativePath),
      }),
    ).toThrow(/anonymous.*\+\+.*no exact 2th next label/i);
  });

  it("does not bless an indirect table without an exact value/domain proof", () => {
    const controlFlow = analyzeRuntimePresentationControlFlow({
      disassemblyRoot,
    });
    const titleChoice = controlFlow.indirect_tables.find(
      (table) => table.table === ".dw@StartTitleScreen",
    );
    expect(titleChoice).toMatchObject({
      entries: [
        "Intro_MainMenu",
        "DeleteSaveData",
        "IntroSequence",
        "IntroSequence",
        "ResetClock",
      ],
      index_domain: { minimum: 0, maximum: 4, values: [0, 1, 2, 3, 4] },
    });

    const intro = canonicalRead(introPath);
    const wrongGuard = intro.replace(
      "DEF NUM_TITLESCREENOPTIONS EQU const_value",
      "DEF NUM_TITLESCREENOPTIONS EQU const_value - 1",
    );
    const mutated = analyzeRuntimePresentationControlFlow({
      disassemblyRoot,
      readSource: (relativePath) =>
        relativePath === introPath ? wrongGuard : canonicalRead(relativePath),
    });
    expect(
      mutated.indirect_tables.find(
        (table) => table.table === ".dw@StartTitleScreen",
      )?.index_domain,
    ).toBeNull();
    expect(() =>
      requireClosedRuntimePresentationControlFlow({
        disassemblyRoot,
        readSource: (relativePath) =>
          relativePath === introPath ? wrongGuard : canonicalRead(relativePath),
      }),
    ).toThrow(/\.dw@StartTitleScreen.*no exact index-domain proof/i);
  });

  it("derives the release main-menu and guarded title-scene domains from source dataflow", () => {
    const controlFlow = analyzeRuntimePresentationControlFlow({
      disassemblyRoot,
    });
    expect(
      controlFlow.indirect_tables.find(
        (table) => table.table === ".Jumptable@MainMenu",
      ),
    ).toMatchObject({
      entries: [
        "MainMenu_Continue",
        "MainMenu_NewGame",
        "MainMenu_Option",
        "MainMenu_MysteryGift",
        "MainMenu_Mobile",
        "MainMenu_MobileStudium",
      ],
      index_domain: { minimum: 0, maximum: 3, values: [0, 1, 2, 3] },
    });
    expect(
      controlFlow.indirect_tables.find(
        (table) => table.table === ".scenes@TitleScreenScene",
      ),
    ).toMatchObject({
      entries: [
        "TitleScreenEntrance",
        "TitleScreenTimer",
        "TitleScreenMain",
        "TitleScreenEnd",
      ],
      index_domain: { minimum: 0, maximum: 3, values: [0, 1, 2, 3] },
    });
  });

  it("rejects a newly reachable sixth main-menu entry from the counted source records", () => {
    const mainMenu = canonicalRead(mainMenuPath);
    const withMobileMysteryItems = replaceExact(
      mainMenu,
      "\t; MAINMENU_MYSTERY\n\tdb 4 + DEF(_DEBUG)\n\tdb MAINMENUITEM_CONTINUE\n\tdb MAINMENUITEM_NEW_GAME\n\tdb MAINMENUITEM_OPTION\n\tdb MAINMENUITEM_MYSTERY_GIFT\n",
      "\t; MAINMENU_MYSTERY\n\tdb 6 + DEF(_DEBUG)\n\tdb MAINMENUITEM_CONTINUE\n\tdb MAINMENUITEM_NEW_GAME\n\tdb MAINMENUITEM_OPTION\n\tdb MAINMENUITEM_MYSTERY_GIFT\n\tdb MAINMENUITEM_MOBILE\n\tdb MAINMENUITEM_MOBILE_STUDIUM\n",
    );
    const mutated = analyzeRuntimePresentationControlFlow({
      disassemblyRoot,
      readSource: (relativePath) =>
        relativePath === mainMenuPath
          ? withMobileMysteryItems
          : canonicalRead(relativePath),
    });
    expect(
      mutated.indirect_tables.find(
        (table) => table.table === ".Jumptable@MainMenu",
      )?.index_domain?.values,
    ).toEqual([0, 1, 2, 3, 4, 5]);
    expect(() =>
      requireClosedRuntimePresentationControlFlow({
        disassemblyRoot,
        readSource: (relativePath) =>
          relativePath === mainMenuPath
            ? withMobileMysteryItems
            : canonicalRead(relativePath),
      }),
    ).toThrow(
      /\.Jumptable@MainMenu.*reachable target MainMenu_Mobile.*index 4/i,
    );
  });

  it("requires the typed scrolling-menu result boundary before using wMenuSelection", () => {
    const mainMenu = canonicalRead(mainMenuPath);
    const withoutTypedSelection = replaceExact(
      mainMenu,
      "\tcall GetScrollingMenuJoypad",
      "\tcall GetJoypad",
    );
    const mutated = analyzeRuntimePresentationControlFlow({
      disassemblyRoot,
      readSource: (relativePath) =>
        relativePath === mainMenuPath
          ? withoutTypedSelection
          : canonicalRead(relativePath),
    });
    expect(
      mutated.indirect_tables.find(
        (table) => table.table === ".Jumptable@MainMenu",
      )?.index_domain,
    ).toBeNull();
  });

  it("rejects an unguarded title-scene index increment", () => {
    const intro = canonicalRead(introPath);
    const unguardedEndIncrement = replaceExact(
      intro,
      "TitleScreenEnd:\n; Wait until the music is done fading.\n\n\tld hl, wTitleScreenTimer\n\tinc [hl]\n\n\tld a, [wMusicFade]\n\tand a\n\tret nz\n\n\tld a, TITLESCREENOPTION_RESTART\n\tld [wTitleScreenSelectedOption], a\n\n; Back to the intro.\n\tld hl, wJumptableIndex\n\tset JUMPTABLE_EXIT_F, [hl]\n\tret\n",
      "TitleScreenEnd:\n; Wait until the music is done fading.\n\n\tld hl, wTitleScreenTimer\n\tinc [hl]\n\n\tld a, [wMusicFade]\n\tand a\n\tret nz\n\n\tld a, TITLESCREENOPTION_RESTART\n\tld [wTitleScreenSelectedOption], a\n\n; Back to the intro.\n\tld hl, wJumptableIndex\n\tinc [hl]\n\tret\n",
    );
    const mutated = analyzeRuntimePresentationControlFlow({
      disassemblyRoot,
      readSource: (relativePath) =>
        relativePath === introPath
          ? unguardedEndIncrement
          : canonicalRead(relativePath),
    });
    expect(
      mutated.indirect_tables.find(
        (table) => table.table === ".scenes@TitleScreenScene",
      )?.index_domain,
    ).toBeNull();
  });

  it("requires the source-ordered title-state zero initializer", () => {
    const title = canonicalRead(titlePath);
    const withoutIndexInitialization = replaceExact(
      title,
      "\tld hl, wJumptableIndex\n\tld [hli], a ; wJumptableIndex",
      "\tld hl, wTitleScreenSelectedOption\n\tld [hli], a ; wTitleScreenSelectedOption",
    );
    const mutated = analyzeRuntimePresentationControlFlow({
      disassemblyRoot,
      readSource: (relativePath) =>
        relativePath === titlePath
          ? withoutIndexInitialization
          : canonicalRead(relativePath),
    });
    expect(
      mutated.indirect_tables.find(
        (table) => table.table === ".scenes@TitleScreenScene",
      )?.index_domain,
    ).toBeNull();
  });

  it("derives the initialized splash sprite callback, frameset, OAM, and outer-state bridge", () => {
    const controlFlow = spriteView(
      analyzeRuntimePresentationControlFlow({ disassemblyRoot }),
    );
    expect(
      controlFlow.sprite_diagnostics.find(
        (entry) => entry.table === ".scenes@GameFreakPresentsScene",
      ),
    ).toBeUndefined();
    expect(controlFlow.sprite_operations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          op: "sprite_init",
          instance: "sprite:engine/movie/splash.asm:93",
          object: { symbol: "SPRITE_ANIM_OBJ_GAMEFREAK_LOGO", value: 3 },
          source_span: {
            file: splashPath,
            start_line: 93,
            end_line: 93,
          },
        }),
        expect.objectContaining({
          op: "sprite_scheduler_step",
          instances: ["sprite:engine/movie/splash.asm:93"],
          source_span: {
            file: splashPath,
            start_line: 47,
            end_line: 47,
          },
          before_host_call: "DelayFrame",
        }),
      ]),
    );
    const program = controlFlow.sprite_programs.find(
      (candidate) => candidate.instance === "sprite:engine/movie/splash.asm:93",
    );
    if (!program) throw new Error("missing compiled splash sprite program");
    expect(
      controlFlow.sprite_programs
        .filter(
          (candidate) => candidate.initializer_source_span.file === splashPath,
        )
        .map((candidate) => candidate.instance),
    ).toEqual(["sprite:engine/movie/splash.asm:93"]);
    expect(program).toMatchObject({
      object: {
        symbol: "SPRITE_ANIM_OBJ_GAMEFREAK_LOGO",
        value: 3,
        table_source_span: {
          file: spriteObjectsPath,
          start_line: 12,
          end_line: 12,
        },
      },
      initial_memory: {
        jumptable_index: 0,
        var1: 96,
        var2: 48,
        frame: 255,
      },
      frameset: {
        symbol: "SPRITE_ANIM_FRAMESET_GAMEFREAK_LOGO",
        value: 10,
        data_source_span: {
          file: spriteFramesetsPath,
          start_line: 142,
          end_line: 158,
        },
        terminal: {
          op: "end",
          source_span: {
            file: spriteFramesetsPath,
            start_line: 158,
            end_line: 158,
          },
        },
      },
      callback: {
        symbol: "SPRITE_ANIM_FUNC_GAMEFREAK_LOGO",
        value: 6,
        wrapper: "SpriteAnimFunc_GameFreakLogo",
        target: "GameFreakLogoSpriteAnim",
        state_table: {
          table: ".scenes@GameFreakLogoSpriteAnim",
          index_domain: {
            minimum: 0,
            maximum: 4,
            values: [0, 1, 2, 3, 4],
          },
        },
      },
      dictionary: { symbol: "SPRITE_ANIM_DICT_DEFAULT", value: 0 },
      callback_data_resources: [
        {
          symbol: "GameFreakDittoPaletteFade",
          kind: "rgb555_palette",
          path: splashPalettePath,
          include_source_span: {
            file: splashPath,
            start_line: 345,
            end_line: 345,
          },
          data_source_span: {
            file: splashPalettePath,
            start_line: 5,
            end_line: 20,
          },
        },
      ],
      outer_state_effects: [
        expect.objectContaining({
          from_callback_index: 3,
          to_callback_index: 4,
          symbol: "wJumptableIndex",
          operation: "increment",
          source_span: {
            file: splashPath,
            start_line: 340,
            end_line: 340,
          },
        }),
      ],
    });
    expect(program.frameset.frames).toHaveLength(15);
    expect(program.oam_resources).toHaveLength(11);
    expect(program.callback_data_resources[0].colors).toHaveLength(16);
    expect(program.callback_data_resources[0].bytes).toHaveLength(32);
    expect(
      controlFlow.indirect_tables.find(
        (table: any) => table.table === ".scenes@GameFreakPresentsScene",
      )?.index_domain?.values,
    ).toEqual([0, 1, 2, 3]);
  });

  it("derives every counter-triggered Intro Unown allocation, override, callback, and deletion lifetime", () => {
    const controlFlow = spriteView(
      analyzeRuntimePresentationControlFlow({ disassemblyRoot }),
    );
    expect(
      controlFlow.sprite_diagnostics.find(
        (entry) => entry.table === "IntroScenes",
      ),
    ).toBeUndefined();
    const unownPrograms = controlFlow.sprite_programs.filter(
      (program) => program.object?.value === 0x29,
    );
    expect(unownPrograms).toHaveLength(12);

    const firstWave = unownPrograms
      .filter((program) => program.allocation_source_span?.start_line === 159)
      .sort(
        (left, right) =>
          left.initializer_source_span.start_line -
          right.initializer_source_span.start_line,
      );
    expect(firstWave).toHaveLength(4);
    expect(
      firstWave.map((program) => ({
        structSlot: program.struct_slot,
        index: program.initial_memory.index,
        x: program.initial_memory.xcoord,
        y: program.initial_memory.ycoord,
        var1: program.initial_memory.var1,
        frameset: program.frameset.value,
      })),
    ).toEqual([
      {
        structSlot: 0,
        index: 1,
        x: 88,
        y: 88,
        var1: 0x08,
        frameset: 0x3c,
      },
      {
        structSlot: 1,
        index: 2,
        x: 88,
        y: 88,
        var1: 0x18,
        frameset: 0x3b,
      },
      {
        structSlot: 2,
        index: 3,
        x: 88,
        y: 88,
        var1: 0x28,
        frameset: 0x39,
      },
      {
        structSlot: 3,
        index: 4,
        x: 88,
        y: 88,
        var1: 0x38,
        frameset: 0x3a,
      },
    ]);
    for (const program of firstWave) {
      expect(program.callback).toMatchObject({
        kind: "direct",
        target: "SpriteAnimFunc_IntroUnown",
        per_tick_struct_deltas: { jumptable_index: 3 },
        host_operations: [
          expect.objectContaining({ op: "sine", target: "AnimSeqs_Sine" }),
          expect.objectContaining({ op: "cosine", target: "AnimSeqs_Cosine" }),
        ],
      });
      expect(program.frameset).toMatchObject({
        terminal: expect.objectContaining({ op: "delete" }),
      });
      expect(program.lifetime).toMatchObject({
        allocation_dispatch_tick: 97,
        scheduler_ticks: 17,
        deinitialized_after_dispatch_tick: 113,
        callback_before_frame_update: true,
      });
    }

    expect(
      unownPrograms.reduce<Record<number, number>>((counts, program) => {
        const line = program.allocation_source_span.start_line;
        counts[line] = (counts[line] ?? 0) + 1;
        return counts;
      }, {}),
    ).toEqual({ 159: 4, 305: 4, 318: 4 });
    expect(
      unownPrograms
        .filter((program) => program.allocation_source_span.start_line === 318)
        .sort(
          (left, right) =>
            left.initializer_source_span.start_line -
            right.initializer_source_span.start_line,
        )
        .map((program) => ({
          structSlot: program.struct_slot,
          index: program.initial_memory.index,
        })),
    ).toEqual([
      { structSlot: 0, index: 5 },
      { structSlot: 1, index: 6 },
      { structSlot: 2, index: 7 },
      { structSlot: 3, index: 8 },
    ]);
    expect(
      controlFlow.sprite_diagnostics.find(
        (entry) => entry.table === "IntroScenes",
      ),
    ).toBeUndefined();
  });

  it.each([
    {
      name: "missing dynamic frameset override",
      file: "engine/movie/intro.asm",
      before:
        "CrystalIntro_InitUnownAnim:\n\tpush de\n\tld a, SPRITE_ANIM_OBJ_INTRO_UNOWN\n\tcall InitSpriteAnimStruct\n\tld hl, SPRITEANIMSTRUCT_VAR1\n\tadd hl, bc\n\tld [hl], $8\n\tld a, SPRITE_ANIM_FRAMESET_INTRO_UNOWN_4\n\tcall ReinitSpriteAnimFrame",
      after:
        "CrystalIntro_InitUnownAnim:\n\tpush de\n\tld a, SPRITE_ANIM_OBJ_INTRO_UNOWN\n\tcall InitSpriteAnimStruct\n\tld hl, SPRITEANIMSTRUCT_VAR1\n\tadd hl, bc\n\tld [hl], $8\n\tld a, SPRITE_ANIM_FRAMESET_INTRO_UNOWN_MISSING\n\tcall ReinitSpriteAnimFrame",
      diagnostic: /frameset.*MISSING|unresolved/i,
    },
    {
      name: "unknown direct callback math host",
      file: spriteFunctionsPath,
      before:
        "SpriteAnimFunc_IntroUnown:\n\tld hl, SPRITEANIMSTRUCT_JUMPTABLE_INDEX\n\tadd hl, bc\n\tld d, [hl]\n\tinc [hl]\n\tinc [hl]\n\tinc [hl]\n\tld hl, SPRITEANIMSTRUCT_VAR1\n\tadd hl, bc\n\tld a, [hl]\n\tpush af\n\tpush de\n\tcall AnimSeqs_Sine",
      after:
        "SpriteAnimFunc_IntroUnown:\n\tld hl, SPRITEANIMSTRUCT_JUMPTABLE_INDEX\n\tadd hl, bc\n\tld d, [hl]\n\tinc [hl]\n\tinc [hl]\n\tinc [hl]\n\tld hl, SPRITEANIMSTRUCT_VAR1\n\tadd hl, bc\n\tld a, [hl]\n\tpush af\n\tpush de\n\tcall UnknownSpriteMathHost",
      diagnostic: /UnknownSpriteMathHost|unsupported.*host/i,
    },
    {
      name: "cosine wrapper loses its exact implementation",
      file: spriteFunctionsPath,
      before: "AnimSeqs_Cosine:\n\tcall Sprites_Cosine\n\tret",
      after: "AnimSeqs_Cosine:\n\tcall Sprites_Sine\n\tret",
      diagnostic: /cosine|Sprites_Cosine/i,
    },
    {
      name: "frame reinitialization no longer clears duration",
      file: spriteCorePath,
      before:
        "_ReinitSpriteAnimFrame::\n\tld hl, SPRITEANIMSTRUCT_FRAMESET_ID\n\tadd hl, bc\n\tld [hl], a\n\tld hl, SPRITEANIMSTRUCT_DURATION\n\tadd hl, bc\n\tld [hl], 0",
      after:
        "_ReinitSpriteAnimFrame::\n\tld hl, SPRITEANIMSTRUCT_FRAMESET_ID\n\tadd hl, bc\n\tld [hl], a\n\tld hl, SPRITEANIMSTRUCT_DURATION\n\tadd hl, bc\n\tld [hl], 1",
      diagnostic: /reinitializer.*frameset.*duration.*frame -1/i,
    },
    {
      name: "next allocation wave loses its prior slot reset",
      file: "engine/movie/intro.asm",
      before:
        "\tfarcall ClearSpriteAnims\n\tcall Intro_SetCGBPalUpdate\n\txor a\n\tld [wIntroSceneFrameCounter], a\n\tld [wIntroSceneTimer], a\n\tcall NextIntroScene\n\tret\n\nIntroScene6:",
      after:
        "\tfarcall ClearSprites\n\tcall Intro_SetCGBPalUpdate\n\txor a\n\tld [wIntroSceneFrameCounter], a\n\tld [wIntroSceneTimer], a\n\tcall NextIntroScene\n\tret\n\nIntroScene6:",
      diagnostic: /IntroScene6.*prior ClearSpriteAnims reset/i,
    },
    {
      name: "Unown frameset no longer deletes its instance",
      file: spriteFramesetsPath,
      before:
        ".Frameset_IntroUnown1:\n\toamframe SPRITE_ANIM_OAMSET_INTRO_UNOWN_1,  3\n\toamframe SPRITE_ANIM_OAMSET_INTRO_UNOWN_2,  3\n\toamframe SPRITE_ANIM_OAMSET_INTRO_UNOWN_3,  7\n\toamdelete",
      after:
        ".Frameset_IntroUnown1:\n\toamframe SPRITE_ANIM_OAMSET_INTRO_UNOWN_1,  3\n\toamframe SPRITE_ANIM_OAMSET_INTRO_UNOWN_2,  3\n\toamframe SPRITE_ANIM_OAMSET_INTRO_UNOWN_3,  7\n\toamend",
      diagnostic: /does not deinitialize|delete lifetime/i,
    },
    {
      name: "oamdelete no longer clears the active struct",
      file: spriteCorePath,
      before:
        ".delete\n; Removes the object from the screen, as opposed to `oamend` which just stops all motion\n\tcall DeinitializeSprite",
      after:
        ".delete\n; Removes the object from the screen, as opposed to `oamend` which just stops all motion\n\tcall ClearSprites",
      diagnostic: /oamdelete.*deinitialization/i,
    },
  ])("fails closed for $name", ({ file, before, after, diagnostic }) => {
    const mutatedSource = replaceExact(canonicalRead(file), before, after);
    const mutated = spriteView(
      analyzeRuntimePresentationControlFlow({
        disassemblyRoot,
        readSource: (relativePath) =>
          relativePath === file ? mutatedSource : canonicalRead(relativePath),
      }),
    );
    expect(
      mutated.sprite_diagnostics.find((entry) => entry.table === "IntroScenes")
        ?.message,
    ).toMatch(diagnostic);
  });

  it.each([
    {
      name: "missing object row",
      file: spriteObjectsPath,
      before:
        "\tdb SPRITE_ANIM_FRAMESET_GAMEFREAK_LOGO,            SPRITE_ANIM_FUNC_GAMEFREAK_LOGO,            SPRITE_ANIM_DICT_DEFAULT",
      after:
        "\tdb SPRITE_ANIM_FRAMESET_GAMEFREAK_LOGO,            SPRITE_ANIM_FUNC_GAMEFREAK_LOGO",
      diagnostic: /object.*row|three fields/i,
    },
    {
      name: "missing function mapping",
      file: spriteFunctionsPath,
      before: "\tdw SpriteAnimFunc_GameFreakLogo",
      after: "\tdw MissingGameFreakSpriteCallback",
      diagnostic: /function.*mapping|MissingGameFreakSpriteCallback/i,
    },
    {
      name: "missing frameset mapping",
      file: spriteObjectsPath,
      before: "SPRITE_ANIM_FRAMESET_GAMEFREAK_LOGO,",
      after: "SPRITE_ANIM_FRAMESET_MISSING_GAMEFREAK_LOGO,",
      diagnostic: /frameset.*mapping|MISSING_GAMEFREAK/i,
    },
    {
      name: "missing OAM mapping",
      file: spriteOamPath,
      before:
        "\tspriteanimoam $d0, .OAMData_GameFreakLogo1_3         ; SPRITE_ANIM_OAMSET_GAMEFREAK_LOGO_1",
      after:
        "\tspriteanimoam $d0, .MissingGameFreakLogoOAM         ; SPRITE_ANIM_OAMSET_GAMEFREAK_LOGO_1",
      diagnostic: /OAM mapping.*missing|MissingGameFreakLogoOAM/i,
    },
    {
      name: "missing sprite reset before initialization",
      file: splashPath,
      before: "\tfarcall ClearSpriteAnims\n\tdepixel 10, 11, 4, 0",
      after: "\tfarcall ClearSprites\n\tdepixel 10, 11, 4, 0",
      diagnostic: /ClearSpriteAnims.*reset/i,
    },
    {
      name: "removed callback outer write",
      file: splashPath,
      before: "\tcall GameFreakPresents_NextScene\nGameFreakLogo_Done:",
      after: "\tld a, a\nGameFreakLogo_Done:",
      diagnostic: /outer.*wJumptableIndex|outer-state/i,
    },
    {
      name: "scheduler after DelayFrame",
      file: splashPath,
      before: "\tfarcall PlaySpriteAnimations\n\tcall DelayFrame",
      after: "\tcall DelayFrame\n\tfarcall PlaySpriteAnimations",
      diagnostic: /scheduler.*DelayFrame|order/i,
    },
    {
      name: "callback after frame update",
      file: spriteCorePath,
      before:
        "DoNextFrameForAllSprites:\n\tld hl, wSpriteAnimationStructs\n\tld e, NUM_SPRITE_ANIM_STRUCTS\n\n.loop\n\tld a, [hl]\n\tand a\n\tjr z, .next ; This struct is deinitialized.\n\tld c, l\n\tld b, h\n\tpush hl\n\tpush de\n\tcall DoSpriteAnimFrame ; Uses a massive dw\n\tcall UpdateAnimFrame",
      after:
        "DoNextFrameForAllSprites:\n\tld hl, wSpriteAnimationStructs\n\tld e, NUM_SPRITE_ANIM_STRUCTS\n\n.loop\n\tld a, [hl]\n\tand a\n\tjr z, .next ; This struct is deinitialized.\n\tld c, l\n\tld b, h\n\tpush hl\n\tpush de\n\tcall UpdateAnimFrame\n\tcall DoSpriteAnimFrame ; Uses a massive dw",
      diagnostic: /callback before frame\/OAM update/i,
    },
    {
      name: "unknown callback opcode",
      file: splashPath,
      before:
        "GameFreakLogo_Init:\n\tld hl, SPRITEANIMSTRUCT_JUMPTABLE_INDEX\n\tadd hl, bc\n\tinc [hl]",
      after:
        "GameFreakLogo_Init:\n\tld hl, SPRITEANIMSTRUCT_JUMPTABLE_INDEX\n\tadd hl, bc\n\tunknowncallback [hl]",
      diagnostic: /unknowncallback|unsupported callback opcode/i,
    },
    {
      name: "unknown callback host call",
      file: splashPath,
      before: "\tcall PlaySFX\n\tret\n\n.done",
      after: "\tcall UnknownSpriteCallbackHost\n\tret\n\n.done",
      diagnostic: /UnknownSpriteCallbackHost|unsupported callback host/i,
    },
    {
      name: "callback palette index outside included data",
      file: splashPath,
      before: "\tcp 64\n\tjr z, .done",
      after: "\tcp 68\n\tjr z, .done",
      diagnostic: /GameFreakDittoPaletteFade.*outside.*32 bytes/i,
    },
  ])("fails closed for $name", ({ file, before, after, diagnostic }) => {
    const mutatedSource = replaceExact(canonicalRead(file), before, after);
    const mutated = spriteView(
      analyzeRuntimePresentationControlFlow({
        disassemblyRoot,
        readSource: (relativePath) =>
          relativePath === file ? mutatedSource : canonicalRead(relativePath),
      }),
    );
    expect(
      mutated.sprite_diagnostics.find(
        (entry) => entry.table === ".scenes@GameFreakPresentsScene",
      )?.message,
    ).toMatch(diagnostic);
    expect(
      mutated.indirect_tables.find(
        (table: any) => table.table === ".scenes@GameFreakPresentsScene",
      )?.index_domain,
    ).toBeNull();
  });

  it("derives the persistent Intro Suicune lifetime across its setup and active handlers", () => {
    const controlFlow = spriteView(
      analyzeRuntimePresentationControlFlow({ disassemblyRoot }),
    );
    expect(controlFlow.sprite_diagnostics).toEqual([]);
    const program = controlFlow.sprite_programs.find(
      (candidate) =>
        candidate.object?.value === 0x26 &&
        candidate.initializer_source_span?.start_line === 393,
    );
    if (!program) throw new Error("canonical Intro Suicune program is missing");

    expect(program).toMatchObject({
      struct_slot: 0,
      initial_memory: {
        index: 1,
        frameset_id: 0x35,
        anim_seq_id: 0x1d,
        xcoord: 216,
        ycoord: 108,
      },
      frameset: {
        value: 0x35,
        terminal: expect.objectContaining({ op: "restart" }),
      },
      callback: {
        kind: "direct",
        target: "SpriteAnimFunc_IntroSuicune",
        outer_memory_reads: [
          expect.objectContaining({
            symbol: "wIntroSceneTimer",
            predicate: "nonzero",
          }),
        ],
        frameset_reinitializations: [
          expect.objectContaining({
            frameset: {
              symbol: "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2",
              value: 0x36,
            },
            guard: expect.objectContaining({
              symbol: "wIntroSceneTimer",
              predicate: "nonzero",
            }),
            application: "every_reachable_scheduler_tick",
            reachable_scheduler_ticks: [],
            source_span: {
              file: spriteFunctionsPath,
              start_line: 775,
              end_line: 775,
            },
          }),
        ],
        host_operations: [
          expect.objectContaining({ op: "sine", target: "AnimSeqs_Sine" }),
        ],
      },
      frameset_variants: [
        expect.objectContaining({
          symbol: "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2",
          value: 0x36,
          terminal: expect.objectContaining({ op: "end" }),
        }),
      ],
      lifetime: {
        allocation_dispatcher_entry: 6,
        allocation_dispatch_tick: 1,
        active_dispatcher_entries: [6, 7],
        scheduler_ticks: 95,
        deinitialized_after_dispatch_tick: 95,
        callback_before_frame_update: true,
        outer_byte_domains: expect.arrayContaining([
          expect.objectContaining({
            symbol: "wIntroSceneTimer",
            initialized_value: 0,
            values: [0],
          }),
          expect.objectContaining({
            symbol: "wGlobalAnimXOffset",
            initialized_value: 0xf0,
            minimum: 0,
            maximum: 0xf0,
          }),
        ]),
        deinitializer: {
          op: "deinitialize_all_sprites",
          dispatcher_entry: 7,
          dispatch_tick: 95,
          before_scheduler_step: true,
          source_span: {
            file: "engine/movie/intro.asm",
            start_line: 428,
            end_line: 428,
          },
          implementation_source_span: {
            file: spriteCorePath,
            start_line: 216,
            end_line: 227,
          },
        },
        handler_host_operations: expect.arrayContaining([
          expect.objectContaining({
            target: "Intro_PerspectiveScrollBG",
            dispatch_ticks: Array.from({ length: 64 }, (_, index) => index + 1),
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 411,
              end_line: 411,
            },
          }),
        ]),
      },
    });
    expect(program.oam_resources).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          oam_set: expect.objectContaining({ value: 0x6e }),
        }),
        expect.objectContaining({
          oam_set: expect.objectContaining({ value: 0x71 }),
        }),
      ]),
    );
  });

  it.each([
    {
      name: "missing callback-selected frameset mapping",
      file: spriteFunctionsPath,
      before:
        "\tld a, SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2\n\tcall _ReinitSpriteAnimFrame",
      after:
        "\tld a, SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_MISSING\n\tcall _ReinitSpriteAnimFrame",
      diagnostic: /frameset.*MISSING|unresolved/i,
    },
    {
      name: "uninitialized callback outer-memory dependency",
      file: spriteFunctionsPath,
      before:
        "SpriteAnimFunc_IntroSuicune:\n\tld a, [wIntroSceneTimer]\n\tand a",
      after:
        "SpriteAnimFunc_IntroSuicune:\n\tld a, [wUninitializedIntroTimer]\n\tand a",
      diagnostic: /wUninitializedIntroTimer.*initialized|outer.*memory/i,
    },
    {
      name: "missing explicit persistent-sprite teardown",
      file: "engine/movie/intro.asm",
      before:
        "\tfarcall DeinitializeAllSprites\n\tcall NextIntroScene\n\tret\n\nIntroScene9:",
      after:
        "\tcall Intro_PerspectiveScrollBG\n\tcall NextIntroScene\n\tret\n\nIntroScene9:",
      diagnostic:
        /persistent.*DeinitializeAllSprites|explicit.*teardown|cross-handler.*not yet source-certified/i,
    },
    {
      name: "bulk deinitializer no longer clears every struct index",
      file: spriteCorePath,
      before:
        "DeinitializeAllSprites:\n; Clear the index field of every struct in the wSpriteAnimationStructs array.\n\tld hl, wSpriteAnimationStructs\n\tld bc, SPRITEANIMSTRUCT_LENGTH\n\tld e, NUM_SPRITE_ANIM_STRUCTS\n\txor a\n.loop\n\tld [hl], a",
      after:
        "DeinitializeAllSprites:\n; Clear the index field of every struct in the wSpriteAnimationStructs array.\n\tld hl, wSpriteAnimationStructs\n\tld bc, SPRITEANIMSTRUCT_LENGTH\n\tld e, NUM_SPRITE_ANIM_STRUCTS\n\txor a\n.loop\n\tinc [hl]",
      diagnostic: /DeinitializeAllSprites.*every struct index|bulk.*deinitial/i,
    },
  ])(
    "fails closed for the Suicune cross-handler proof when $name",
    ({ file, before, after, diagnostic }) => {
      const mutatedSource = replaceExact(canonicalRead(file), before, after);
      const mutated = spriteView(
        analyzeRuntimePresentationControlFlow({
          disassemblyRoot,
          readSource: (relativePath) =>
            relativePath === file ? mutatedSource : canonicalRead(relativePath),
        }),
      );
      expect(
        mutated.sprite_diagnostics.find(
          (entry) => entry.table === "IntroScenes",
        )?.message,
      ).toMatch(diagnostic);
    },
  );

  it("derives the Scene10 Wooper/Pichu allocation count, callback domains, terminal frames, and next-scene clear", () => {
    const controlFlow = spriteView(
      analyzeRuntimePresentationControlFlow({ disassemblyRoot }),
    );
    const byLine = (line: number) => {
      const program = controlFlow.sprite_programs.find(
        (candidate) => candidate.initializer_source_span?.start_line === line,
      );
      if (!program)
        throw new Error(`canonical sprite program ${line} is missing`);
      return program;
    };
    const wooper = byLine(494);
    const pichu = byLine(486);

    expect(wooper).toMatchObject({
      struct_slot: 0,
      object: { value: 0x28 },
      initial_memory: {
        index: 2,
        frameset_id: 0x38,
        anim_seq_id: 0x1e,
        xcoord: 48,
        ycoord: 176,
        var1: 0,
      },
      frameset: {
        terminal: expect.objectContaining({ op: "end" }),
        frames: [expect.objectContaining({ duration: 3 })],
      },
      callback: {
        kind: "direct",
        target: "SpriteAnimFunc_IntroPichuWooper",
        labels: { ".done": 14 },
        host_operations: [
          expect.objectContaining({ op: "sine", target: "AnimSeqs_Sine" }),
        ],
        struct_control_byte_domains: [
          {
            property: "var1",
            initialized_value: 0,
            minimum: 0,
            maximum: 20,
            values: [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20],
          },
        ],
      },
      lifetime: {
        allocation_dispatcher_entry: 9,
        allocation_dispatch_tick: 33,
        active_dispatcher_entries: [9, 10],
        scheduler_ticks: 161,
        deinitialized_after_dispatch_tick: 1,
        outer_byte_domains: expect.arrayContaining([
          expect.objectContaining({
            symbol: "wIntroSceneFrameCounter",
            initialized_value: 0,
            minimum: 0,
            maximum: 193,
          }),
        ]),
        deinitializer: expect.objectContaining({
          op: "clear_sprite_anims",
          dispatcher_entry: 10,
          dispatch_tick: 1,
          before_scheduler_step: true,
          source_span: {
            file: "engine/movie/intro.asm",
            start_line: 544,
            end_line: 544,
          },
          implementation_source_span: {
            file: spriteCorePath,
            start_line: 1,
            end_line: 11,
          },
        }),
      },
    });
    expect(wooper.oam_resources).toEqual([
      expect.objectContaining({
        oam_set: expect.objectContaining({ value: 0x75 }),
      }),
    ]);

    expect(pichu).toMatchObject({
      struct_slot: 1,
      object: { value: 0x27 },
      initial_memory: {
        index: 3,
        frameset_id: 0x37,
        anim_seq_id: 0x1e,
        xcoord: 128,
        ycoord: 169,
        var1: 0,
      },
      frameset: {
        terminal: expect.objectContaining({ op: "end" }),
        frames: [
          expect.objectContaining({ duration: 32 }),
          expect.objectContaining({ duration: 7 }),
          expect.objectContaining({ duration: 7 }),
        ],
      },
      callback: {
        struct_control_byte_domains: [
          expect.objectContaining({
            property: "var1",
            values: [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20],
          }),
        ],
      },
      lifetime: {
        allocation_dispatcher_entry: 9,
        allocation_dispatch_tick: 65,
        active_dispatcher_entries: [9, 10],
        scheduler_ticks: 129,
        deinitialized_after_dispatch_tick: 1,
        deinitializer: expect.objectContaining({
          op: "clear_sprite_anims",
          dispatcher_entry: 10,
          dispatch_tick: 1,
        }),
      },
    });
    expect(
      pichu.oam_resources.map((entry: any) => entry.oam_set.value),
    ).toEqual([0x72, 0x73, 0x74]);
  });

  it.each([
    {
      name: "callback control opcode is unsupported",
      file: spriteFunctionsPath,
      before:
        "SpriteAnimFunc_IntroPichuWooper:\n\tld hl, SPRITEANIMSTRUCT_VAR1\n\tadd hl, bc\n\tld a, [hl]\n\tcp 20",
      after:
        "SpriteAnimFunc_IntroPichuWooper:\n\tld hl, SPRITEANIMSTRUCT_VAR1\n\tadd hl, bc\n\tld a, [hl]\n\tunknowncallbackcap 20",
      diagnostic:
        /unknowncallbackcap|unsupported direct sprite callback opcode/i,
    },
    {
      name: "Wooper frameset has an unknown persistent terminal",
      file: spriteFramesetsPath,
      before:
        ".Frameset_IntroWooper:\n\toamframe SPRITE_ANIM_OAMSET_INTRO_WOOPER,  3\n\toamend",
      after:
        ".Frameset_IntroWooper:\n\toamframe SPRITE_ANIM_OAMSET_INTRO_WOOPER,  3\n\toamunknown",
      diagnostic: /oamunknown|unsupported sprite frameset opcode/i,
    },
    {
      name: "Scene11 omits the full animation-state clear",
      file: "engine/movie/intro.asm",
      before:
        "\tfarcall ClearSpriteAnims\n\tcall Intro_SetCGBPalUpdate\n\txor a\n\tld [wIntroSceneFrameCounter], a\n\tld [wIntroSceneTimer], a\n\tcall NextIntroScene\n\tret\n\nIntroScene12:",
      after:
        "\tcall ClearSprites\n\tcall Intro_SetCGBPalUpdate\n\txor a\n\tld [wIntroSceneFrameCounter], a\n\tld [wIntroSceneTimer], a\n\tcall NextIntroScene\n\tret\n\nIntroScene12:",
      diagnostic:
        /Scene10|IntroScene10|persistent.*ClearSpriteAnims|next-scene clear/i,
    },
  ])(
    "fails closed for Scene10 when $name",
    ({ file, before, after, diagnostic }) => {
      const mutatedSource = replaceExact(canonicalRead(file), before, after);
      const mutated = spriteView(
        analyzeRuntimePresentationControlFlow({
          disassemblyRoot,
          readSource: (relativePath) =>
            relativePath === file ? mutatedSource : canonicalRead(relativePath),
        }),
      );
      expect(
        mutated.sprite_diagnostics.find(
          (entry) => entry.table === "IntroScenes",
        )?.message,
      ).toMatch(diagnostic);
    },
  );

  it("derives Scene13/14 high-memory scroll, repeated Suicune reinitialization, teardown, and later scene advance", () => {
    const controlFlow = spriteView(
      analyzeRuntimePresentationControlFlow({ disassemblyRoot }),
    );
    const program = controlFlow.sprite_programs.find(
      (candidate) => candidate.initializer_source_span?.start_line === 673,
    );
    if (!program)
      throw new Error("canonical Scene13 Suicune program is missing");

    expect(program).toMatchObject({
      struct_slot: 0,
      object: { value: 0x26 },
      initial_memory: {
        index: 1,
        frameset_id: 0x35,
        xcoord: 88,
        ycoord: 108,
        var2: 0,
      },
      callback: {
        kind: "direct",
        target: "SpriteAnimFunc_IntroSuicune",
        labels: { ".continue": 4 },
        frameset_reinitializations: [
          expect.objectContaining({
            application: "every_reachable_scheduler_tick",
            frameset: {
              symbol: "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2",
              value: 0x36,
            },
            reachable_scheduler_ticks: [98, 99, 100, 101, 102, 103, 104, 105],
          }),
        ],
      },
      frameset_variants: [
        expect.objectContaining({
          value: 0x36,
          terminal: expect.objectContaining({ op: "end" }),
        }),
      ],
      lifetime: {
        allocation_dispatcher_entry: 12,
        allocation_dispatch_tick: 1,
        active_dispatcher_entries: [12, 13],
        scheduler_ticks: 105,
        deinitialized_after_dispatch_tick: 105,
        outer_byte_domains: expect.arrayContaining([
          expect.objectContaining({
            symbol: "wIntroSceneTimer",
            initialized_value: 0,
            values: [0, 1],
          }),
          expect.objectContaining({
            symbol: "wIntroSceneFrameCounter",
            initialized_value: 0,
            minimum: 0,
            maximum: 129,
          }),
          expect.objectContaining({
            symbol: "hSCX",
            initialized_value: 0,
            minimum: 0,
            maximum: 254,
            values: Array.from({ length: 128 }, (_, index) => index * 2),
          }),
        ]),
        deinitializer: expect.objectContaining({
          op: "deinitialize_all_sprites",
          dispatcher_entry: 13,
          dispatch_tick: 105,
          reachable_dispatch_ticks: Array.from(
            { length: 24 },
            (_, index) => index + 105,
          ),
          source_span: {
            file: "engine/movie/intro.asm",
            start_line: 717,
            end_line: 717,
          },
        }),
        outer_scene_advances: [
          {
            dispatcher_entry: 13,
            dispatch_tick: 129,
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 727,
              end_line: 727,
            },
          },
        ],
      },
    });
  });

  it.each([
    {
      name: "high-memory scroll source is uninitialized",
      before: "IntroScene14:\n; Suicune runs then jumps.\n\tldh a, [hSCX]",
      after:
        "IntroScene14:\n; Suicune runs then jumps.\n\tldh a, [hUninitializedIntroScroll]",
      diagnostic: /hUninitializedIntroScroll.*initialized|outer memory/i,
    },
    {
      name: "persistent sprite teardown target is unsupported",
      before: ".disappear\n\tfarcall DeinitializeAllSprites\n\tret\n\n.run",
      after: ".disappear\n\tfarcall UnknownSpriteDeinitializer\n\tret\n\n.run",
      diagnostic: /UnknownSpriteDeinitializer|unresolved.*host/i,
    },
    {
      name: "dynamic run threshold is unresolved",
      before: "\tld a, [wGlobalAnimXOffset]\n\tcp $88\n\tjr c, .disappear",
      after:
        "\tld a, [wGlobalAnimXOffset]\n\tcp UNKNOWN_SUICUNE_THRESHOLD\n\tjr c, .disappear",
      diagnostic: /UNKNOWN_SUICUNE_THRESHOLD|ASM integer/i,
    },
  ])(
    "fails closed for Scene13/14 when $name",
    ({ before, after, diagnostic }) => {
      const file = "engine/movie/intro.asm";
      const mutatedSource = replaceExact(canonicalRead(file), before, after);
      const mutated = spriteView(
        analyzeRuntimePresentationControlFlow({
          disassemblyRoot,
          readSource: (relativePath) =>
            relativePath === file ? mutatedSource : canonicalRead(relativePath),
        }),
      );
      expect(
        mutated.sprite_diagnostics.find(
          (entry) => entry.table === "IntroScenes",
        )?.message,
      ).toMatch(diagnostic);
    },
  );

  it("derives Scene15/16's two object-local programs, WRAM alias guard, wait frame, and next-scene clear", () => {
    const controlFlow = spriteView(
      analyzeRuntimePresentationControlFlow({ disassemblyRoot }),
    );
    const unown = controlFlow.sprite_programs.find(
      (candidate) => candidate.initializer_source_span?.start_line === 784,
    );
    const suicune = controlFlow.sprite_programs.find(
      (candidate) => candidate.initializer_source_span?.start_line === 787,
    );
    if (!unown || !suicune) {
      throw new Error("canonical Scene15 sprite programs are missing");
    }

    expect(unown).toMatchObject({
      struct_slot: 0,
      object: { value: 0x2a },
      initial_memory: {
        index: 1,
        frameset_id: 0x3f,
        anim_seq_id: 0x21,
        xcoord: 40,
        ycoord: 64,
      },
      frameset: {
        frames: [],
        waits: [
          expect.objectContaining({
            duration: 0,
            source_span: {
              file: spriteFramesetsPath,
              start_line: 488,
              end_line: 488,
            },
            implementation_source_span: {
              file: "macros/scripts/oam_anims.asm",
              start_line: 29,
              end_line: 33,
            },
          }),
        ],
        terminal: expect.objectContaining({ op: "end" }),
      },
      callback: {
        kind: "direct",
        target: "SpriteAnimFunc_IntroUnownF",
        outer_memory_reads: [
          expect.objectContaining({
            source_symbol: "wSlotsDelay",
            symbol: "wIntroSceneFrameCounter",
            predicate: "equals",
            comparison_value: 0x40,
            alias_source_spans: expect.arrayContaining([
              expect.objectContaining({ start_line: 1516 }),
              expect.objectContaining({ start_line: 1563 }),
            ]),
          }),
        ],
        frameset_reinitializations: [
          expect.objectContaining({
            frameset: {
              symbol: "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_F_2",
              value: 0x3d,
            },
            guard: expect.objectContaining({
              source_symbol: "wSlotsDelay",
              symbol: "wIntroSceneFrameCounter",
              predicate: "equals",
              comparison_value: 0x40,
            }),
            reachable_scheduler_ticks: [65],
          }),
        ],
      },
      lifetime: {
        allocation_dispatcher_entry: 14,
        allocation_dispatch_tick: 1,
        active_dispatcher_entries: [14, 15, 16],
        scheduler_ticks: 130,
        deinitialized_after_dispatch_tick: 1,
        outer_byte_domains: expect.arrayContaining([
          expect.objectContaining({
            symbol: "wIntroSceneFrameCounter",
            initialized_value: 0,
            minimum: 0,
            maximum: 129,
          }),
          expect.objectContaining({
            symbol: "hSCY",
            initialized_value: 144,
            values: [
              0, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216, 224, 232,
              240, 248,
            ],
          }),
        ]),
        deinitializer: expect.objectContaining({
          op: "clear_sprite_anims",
          dispatcher_entry: 16,
          dispatch_tick: 1,
          source_span: {
            file: "engine/movie/intro.asm",
            start_line: 853,
            end_line: 853,
          },
        }),
        outer_scene_advances: [
          {
            dispatcher_entry: 15,
            dispatch_tick: 129,
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 809,
              end_line: 809,
            },
          },
        ],
        handler_host_operations: [
          expect.objectContaining({
            target: "Intro_Scene16_AnimateSuicune",
            dispatch_ticks: Array.from(
              { length: 128 },
              (_, index) => index + 1,
            ),
          }),
        ],
      },
    });
    expect(unown.frameset_variants).toEqual([
      expect.objectContaining({
        value: 0x3d,
        frames: expect.arrayContaining([
          expect.objectContaining({ duration: 3 }),
          expect.objectContaining({ duration: 7 }),
        ]),
        terminal: expect.objectContaining({ op: "end" }),
      }),
    ]);

    expect(suicune).toMatchObject({
      struct_slot: 1,
      object: { value: 0x2b },
      initial_memory: {
        index: 2,
        frameset_id: 0x3e,
        anim_seq_id: 0x22,
        xcoord: 0,
        ycoord: 96,
      },
      callback: {
        kind: "direct",
        target: "SpriteAnimFunc_IntroSuicuneAway",
        per_tick_struct_deltas: { ycoord: 16 },
        struct_control_byte_domains: [
          expect.objectContaining({
            property: "ycoord",
            initialized_value: 96,
            values: Array.from({ length: 16 }, (_, index) => index * 16),
          }),
        ],
      },
      lifetime: {
        allocation_dispatcher_entry: 14,
        allocation_dispatch_tick: 1,
        active_dispatcher_entries: [14, 15, 16],
        scheduler_ticks: 130,
        deinitialized_after_dispatch_tick: 1,
        deinitializer: expect.objectContaining({
          op: "clear_sprite_anims",
          dispatcher_entry: 16,
          dispatch_tick: 1,
        }),
      },
    });
  });

  it.each([
    {
      name: "the callback reads an undeclared WRAM alias",
      file: spriteFunctionsPath,
      before: "SpriteAnimFunc_IntroUnownF:\n\tld a, [wSlotsDelay]",
      after: "SpriteAnimFunc_IntroUnownF:\n\tld a, [wUnknownIntroAlias]",
      diagnostic: /wUnknownIntroAlias.*WRAM.*alias|outer memory.*initialized/i,
    },
    {
      name: "the base frameset has an unsupported wait operation",
      file: spriteFramesetsPath,
      before: ".Frameset_IntroUnownF:\n\toamwait 0\n\toamend",
      after: ".Frameset_IntroUnownF:\n\toamwait_unknown 0\n\toamend",
      diagnostic: /oamwait_unknown|unsupported sprite frameset opcode/i,
    },
    {
      name: "the engine no longer routes oamwait through its no-OAM path",
      file: spriteCorePath,
      before: "\tcp oamwait_command\n\tjr z, .done\n\tcp oamdelete_command",
      after: "\tcp oamwait_command\n\tjr nz, .done\n\tcp oamdelete_command",
      diagnostic: /oamwait.*exact.*done|oamwait.*skip OAM/i,
    },
    {
      name: "Scene17 omits the exact full animation-state clear",
      file: "engine/movie/intro.asm",
      before:
        "\tldh [hWY], a\n\tfarcall ClearSpriteAnims\n\tcall Intro_SetCGBPalUpdate\n\txor a\n\tld [wIntroSceneFrameCounter], a\n\tld [wIntroSceneTimer], a\n\tcall NextIntroScene\n\tret\n\nIntroScene18:",
      after:
        "\tldh [hWY], a\n\tcall ClearSprites\n\tcall Intro_SetCGBPalUpdate\n\txor a\n\tld [wIntroSceneFrameCounter], a\n\tld [wIntroSceneTimer], a\n\tcall NextIntroScene\n\tret\n\nIntroScene18:",
      diagnostic:
        /Scene16|persistent.*ClearSpriteAnims|next-scene clear|cross-handler.*rWBK|outer memory.*rWBK/i,
    },
    {
      name: "the WRAM union moves the callback byte away from the intro counter",
      file: wramPath,
      before: "NEXTU\n; slot machine\nwSlotsDelay:: db",
      after: "; slot machine\nwSlotsDelay:: db",
      diagnostic: /wSlotsDelay.*alias|outer memory.*initialized/i,
    },
  ])(
    "fails closed for Scene15/16 when $name",
    ({ file, before, after, diagnostic }) => {
      const mutatedSource = replaceExact(canonicalRead(file), before, after);
      const mutated = spriteView(
        analyzeRuntimePresentationControlFlow({
          disassemblyRoot,
          readSource: (relativePath) =>
            relativePath === file ? mutatedSource : canonicalRead(relativePath),
        }),
      );
      expect(
        mutated.sprite_diagnostics.find(
          (entry) => entry.table === "IntroScenes",
        )?.message,
      ).toMatch(diagnostic);
    },
  );

  it("derives Scene19-22's dictionary tile, cross-handler lifetime, and internal VBlank waits", () => {
    const controlFlow = spriteView(
      analyzeRuntimePresentationControlFlow({ disassemblyRoot }),
    );
    const program = controlFlow.sprite_programs.find(
      (candidate) => candidate.initializer_source_span?.start_line === 936,
    );
    if (!program) {
      throw new Error("canonical Scene19 Suicune-away program is missing");
    }

    expect(program).toMatchObject({
      struct_slot: 0,
      object: { value: 0x2b },
      initial_memory: {
        index: 1,
        frameset_id: 0x3e,
        anim_seq_id: 0x22,
        tile_id: 0x7f,
        xcoord: 0,
        ycoord: 96,
      },
      callback: {
        kind: "direct",
        target: "SpriteAnimFunc_IntroSuicuneAway",
        per_tick_struct_deltas: { ycoord: 16 },
        struct_control_byte_domains: [
          expect.objectContaining({
            property: "ycoord",
            initialized_value: 96,
            values: Array.from({ length: 16 }, (_, index) => index * 16),
          }),
        ],
      },
      lifetime: {
        allocation_dispatcher_entry: 18,
        allocation_dispatch_tick: 1,
        active_dispatcher_entries: [18, 19, 20, 21],
        scheduler_ticks: 163,
        deinitialized_after_dispatch_tick: 9,
        outer_byte_domains: expect.arrayContaining([
          expect.objectContaining({
            symbol: "wIntroSceneFrameCounter",
            initialized_value: 0,
            minimum: 0,
            maximum: 153,
          }),
          expect.objectContaining({
            symbol: "hSCY",
            initialized_value: 216,
            values: [
              0,
              ...Array.from({ length: 40 }, (_, index) => index + 216),
            ],
          }),
        ]),
        deinitializer: expect.objectContaining({
          op: "deinitialize_all_sprites",
          dispatcher_entry: 21,
          dispatch_tick: 9,
          source_span: {
            file: "engine/movie/intro.asm",
            start_line: 1010,
            end_line: 1010,
          },
        }),
        outer_scene_advances: [
          {
            dispatcher_entry: 19,
            dispatch_tick: 153,
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 987,
              end_line: 987,
            },
          },
          {
            dispatcher_entry: 20,
            dispatch_tick: 1,
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 999,
              end_line: 999,
            },
          },
          {
            dispatcher_entry: 21,
            dispatch_tick: 9,
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 1011,
              end_line: 1011,
            },
          },
        ],
        pre_scheduler_waits: [
          {
            target: "DelayFrames",
            dispatcher_entry: 20,
            dispatch_tick: 1,
            frame_count: 3,
            before_scheduler_step: true,
            source_span: {
              file: "engine/movie/intro.asm",
              start_line: 994,
              end_line: 994,
            },
            implementation_source_span: {
              file: delayPath,
              start_line: 1,
              end_line: 20,
            },
          },
        ],
        handler_host_operations: expect.arrayContaining([
          expect.objectContaining({
            target: "Intro_Scene20_AppearUnown",
            dispatcher_entry: 19,
            dispatch_ticks: [68, 72, 76, 80, 84, 88],
          }),
          expect.objectContaining({
            target: "Intro_ColoredSuicuneFrameSwap",
            dispatcher_entry: 20,
            dispatch_ticks: [1],
          }),
        ]),
      },
    });
    expect(program.oam_resources).toEqual([
      expect.objectContaining({
        oam_set: expect.objectContaining({ value: 0x7e }),
        tile_offset: 0x80,
      }),
    ]);
  });

  it.each([
    {
      name: "DelayFrames no longer loops through the exact one-frame wait",
      file: delayPath,
      before: "DelayFrames::\n; Wait c frames\n\tcall DelayFrame",
      after: "DelayFrames::\n; Wait c frames\n\tcall DelayFrameOpaque",
      diagnostic: /DelayFrames|DelayFrameOpaque|internal.*wait/i,
    },
    {
      name: "the sprite dictionary tile is not a source byte",
      file: "engine/movie/intro.asm",
      before: "\tld [hli], a\n\tld [hl], $7f\n\tcall Intro_SetCGBPalUpdate",
      after:
        "\tld [hli], a\n\tld [hl], UNKNOWN_INTRO_DICT_TILE\n\tcall Intro_SetCGBPalUpdate",
      diagnostic: /UNKNOWN_INTRO_DICT_TILE|dictionary.*tile|ASM integer/i,
    },
    {
      name: "an internal wait invokes the sprite scheduler",
      file: "engine/movie/intro.asm",
      before: "\tld c, 3\n\tcall DelayFrames\n\txor a",
      after: "\tld c, 3\n\tfarcall PlaySpriteAnimations\n\txor a",
      diagnostic: /PlaySpriteAnimations|nested.*scheduler|internal.*wait/i,
    },
    {
      name: "Scene22 removes the exact sprite teardown",
      file: "engine/movie/intro.asm",
      before:
        ".done\n\tfarcall DeinitializeAllSprites\n\tcall NextIntroScene\n\tret\n\nIntroScene23:",
      after:
        ".done\n\tfarcall UnknownSpriteTeardown\n\tcall NextIntroScene\n\tret\n\nIntroScene23:",
      diagnostic: /UnknownSpriteTeardown|unresolved.*host|teardown/i,
    },
  ])(
    "fails closed for Scene19-22 when $name",
    ({ file, before, after, diagnostic }) => {
      const mutatedSource = replaceExact(canonicalRead(file), before, after);
      const mutated = spriteView(
        analyzeRuntimePresentationControlFlow({
          disassemblyRoot,
          readSource: (relativePath) =>
            relativePath === file ? mutatedSource : canonicalRead(relativePath),
        }),
      );
      expect(
        mutated.sprite_diagnostics.find(
          (entry) => entry.table === "IntroScenes",
        )?.message,
      ).toMatch(diagnostic);
    },
  );

  it("closes every source-reachable title and intro sprite program", () => {
    const controlFlow = requireClosedRuntimePresentationControlFlow({
      disassemblyRoot,
    });
    expect(controlFlow.sprite_diagnostics).toEqual([]);
  });
});
