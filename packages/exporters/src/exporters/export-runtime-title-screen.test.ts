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
    expect(checkpoint.frontier).toEqual({
      reason: "missing_subprogram_contract",
      block: ".loop@CrystalIntro",
      target: "JoyTextDelay",
      opcode: "call",
      args: ["JoyTextDelay"],
      source_span: {
        file: "engine/movie/intro.asm",
        start_line: 12,
        end_line: 12,
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
            instances: [],
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
        ]),
      },
    });
    const introPrefix = checkpoint.frontier?.compiled_prefix?.operations ?? [];
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
    ]);

    const subprograms = (checkpoint as any).subprograms as Array<any>;
    expect(subprograms).toHaveLength(1);
    expect(subprograms[0]).toMatchObject({
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

  it("fails closed at the next reachable callable without an exact typed contract", () => {
    expect(() =>
      buildRuntimeTitlePresentationProgram({
        disassemblyRoot,
        audioAssetIds: new Set(),
        runtimeSpawnIdentifiers: new Set([0]),
      }),
    ).toThrow(
      /source call JoyTextDelay.*engine\/movie\/intro\.asm:12.*typed subprogram contract/i,
    );
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
      before: "\tjr nc, .done\n\txor a\n\tld [hli], a\n\tjr .loop2",
      after: "\tjr nc, .done\n\tinc a\n\tld [hli], a\n\tjr .loop2",
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
    const helperWrite = checkpoint.frontier?.compiled_prefix?.operations.find(
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
        checkpoint.frontier?.compiled_prefix?.operations.filter((operation) =>
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
        checkpoint.frontier?.compiled_prefix?.operations.filter(
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
