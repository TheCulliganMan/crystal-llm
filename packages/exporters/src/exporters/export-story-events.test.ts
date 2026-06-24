import fs from "fs";
import os from "os";
import path from "path";
import { parseAsmFile } from "./export-story-events";

describe("export-story-events", () => {
  it("preserves scene script order", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "Route29.asm");
    fs.writeFileSync(
      asmSource,
      `Route29_MapScripts:
\tdef_scene_scripts
\tscene_script Route29Noop1Scene, SCENE_ROUTE29_NOOP
\tscene_script Route29Noop2Scene, SCENE_ROUTE29_CATCH_TUTORIAL
\tdef_callbacks
\tcallback MAPCALLBACK_OBJECTS, Route29TuscanyCallback

Route29Noop1Scene:
\tend

Route29Noop2Scene:
\tend
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);
    const sceneScripts = scripts.Route29_MapScripts.filter((command) => command.command === "scene_script");
    expect(sceneScripts.map((command) => (command.args as string[])[1])).toEqual([
      "SCENE_ROUTE29_NOOP",
      "SCENE_ROUTE29_CATCH_TUTORIAL",
    ]);
  });

  it("strips inline comments from text commands", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "CommentedText.asm");
    fs.writeFileSync(
      asmSource,
      `CommentedTextScript:
\ttext "HELLO" ; "ignored comment"
\tline "WORLD" ; another comment
\tdone
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);
    const commands = scripts.CommentedTextScript;
    expect(commands[0].args).toBe('"HELLO"');
    expect(commands[1].args).toBe('"WORLD"');
    expect(commands[2].command).toBe("done");
  });

  it("keeps the non-AU branch of conditional map text labels", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "GoldenrodGameCorner.asm");
    fs.writeFileSync(
      asmSource,
      `GoldenrodGameCornerPokefanM2Text:
if DEF(_CRYSTAL_AU)
\ttext "COIN CASE? I threw"
\tline "it away."
\tdone
else
\ttext "I couldn't win at"
\tline "the slots, and I"
\tpara "blew it on card"
\tline "flipping..."
\tdone
endc
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts.GoldenrodGameCornerPokefanM2Text).toEqual([
      { command: "text", args: '"I couldn\'t win at"' },
      { command: "line", args: '"the slots, and I"' },
      { command: "para", args: '"blew it on card"' },
      { command: "line", args: '"flipping..."' },
      { command: "done", args: "" },
    ]);
  });

  it("materializes the lower Azalea rival scene fallthrough", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "AzaleaTown.asm");
    fs.writeFileSync(
      asmSource,
      `AzaleaTownRivalBattleScene2:
\tturnobject PLAYER, UP
AzaleaTownRivalBattleScript:
\tplaymusic MUSIC_RIVAL_ENCOUNTER
\tend
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts.AzaleaTownRivalBattleScene2.at(-1)).toEqual({
      command: "sjump",
      args: ["AzaleaTownRivalBattleScript"],
    });
  });

  it("preserves consecutive local labels as aliases for the same commands", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "IlexForest.asm");
    fs.writeFileSync(
      asmSource,
      `IlexForestFarfetchdScript:
\tifequal UP, .Position8_Up
\tifequal LEFT, .Position8_Left
\tend

.Position8_Up:
.Position8_Left:
\tapplymovement ILEXFOREST_FARFETCHD, MovementData_Farfetched_Pos8_Pos2
\tend
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts[".Position8_Up@IlexForestFarfetchdScript"]).toEqual([
      {
        command: "applymovement",
        args: ["ILEXFOREST_FARFETCHD", "MovementData_Farfetched_Pos8_Pos2"],
      },
      { command: "end", args: [] },
    ]);
    expect(scripts[".Position8_Left@IlexForestFarfetchdScript"]).toBe(
      scripts[".Position8_Up@IlexForestFarfetchdScript"]
    );
  });

  it("expands Goldenrod underground switch door macros into local scripts", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "GoldenrodUndergroundSwitchRoomEntrances.asm");
    fs.writeFileSync(
      asmSource,
      `DEF ugdoor_n = 0

MACRO ugdoor_def
ENDM

\tugdoor_def 16,  6,    $3e,  $2d
\tugdoor_def 12,  6,    $3f,  $2a, 12,  8,    $3d,  $2d

GoldenrodUndergroundSwitchRoomEntrances_UpdateDoors:
\tscall .OpenDoor1
\tscall .CloseDoor2
\tend

for n, 1, ugdoor_n + 1
.OpenDoor{d:n}:
\tchangeugdoor n, OPEN
\tsetevent EVENT_DOOR_{d:n}_OPEN
\tend
endr

for n, 1, ugdoor_n + 1
.CloseDoor{d:n}:
\tchangeugdoor n, CLOSED
\tclearevent EVENT_DOOR_{d:n}_OPEN
\tend
endr
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts[".OpenDoor1@GoldenrodUndergroundSwitchRoomEntrances_UpdateDoors"]).toEqual([
      { command: "changeblock", args: ["16", "6", "$2d"] },
      { command: "setevent", args: ["EVENT_DOOR_1_OPEN"] },
      { command: "end", args: [] },
    ]);
    expect(scripts[".CloseDoor2@GoldenrodUndergroundSwitchRoomEntrances_UpdateDoors"]).toEqual([
      { command: "changeblock", args: ["12", "6", "$3f"] },
      { command: "changeblock", args: ["12", "8", "$3d"] },
      { command: "clearevent", args: ["EVENT_DOOR_2_OPEN"] },
      { command: "end", args: [] },
    ]);
  });

  it("expands Goldenrod underground switch callback door loops", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "GoldenrodUndergroundSwitchRoomEntrances.asm");
    fs.writeFileSync(
      asmSource,
      `DEF ugdoor_n = 0

MACRO ugdoor_def
ENDM

\tugdoor_def 16,  6,    $3e,  $2d
\tugdoor_def 12,  6,    $3f,  $2a, 12,  8,    $3d,  $2d

GoldenrodUndergroundSwitchRoomEntrancesUpdateDoorPositionsCallback:
for n, 1, ugdoor_n + 1
\tcheckevent EVENT_DOOR_{d:n}_OPEN
\tiffalse .door_{d:n}_closed
\tchangeugdoor n, OPEN
.door_{d:n}_closed
endr
\tendcallback
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts.GoldenrodUndergroundSwitchRoomEntrancesUpdateDoorPositionsCallback).toEqual([
      { command: "checkevent", args: ["EVENT_DOOR_1_OPEN"] },
      { command: "iffalse", args: [".door_1_closed"] },
      { command: "changeblock", args: ["16", "6", "$2d"] },
      { command: "checkevent", args: ["EVENT_DOOR_2_OPEN"] },
      { command: "iffalse", args: [".door_2_closed"] },
      { command: "changeblock", args: ["12", "6", "$2a"] },
      { command: "changeblock", args: ["12", "8", "$2d"] },
      { command: "endcallback", args: [] },
    ]);
  });
});
