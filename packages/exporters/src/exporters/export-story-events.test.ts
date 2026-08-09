import fs from "fs";
import os from "os";
import path from "path";
import {
  isExecutableScriptControlEdge,
  isExecutableScriptControlTargetBody,
  isUnconditionalScriptTransferCommand,
  parseAsmFile,
  parseStandardScriptsFile,
  standardScriptsStoryEventPayload,
} from "./export-story-events";

describe("export-story-events", () => {
  it("exports the exact standard-script pointer order with parsed command bodies", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "standard-scripts-"));
    const asmSource = path.join(tmpDir, "std_scripts.asm");
    fs.writeFileSync(
      asmSource,
      `MACRO add_stdscript
\\1StdScript::
\tdba \\1
ENDM

StdScripts::
\tadd_stdscript SignScript
\tadd_stdscript NurseScript

SignScript:
\tfarjumptext SignText

NurseScript:
\topentext
\tyesorno
\tiffalse .Done
\tspecial HealParty
.Done:
\tclosetext
\tend
`,
      "utf8",
    );

    const payload = parseStandardScriptsFile(asmSource);

    expect(payload.order).toEqual(["SignScript", "NurseScript"]);
    expect(payload.scripts.SignScript).toEqual([
      { command: "farjumptext", args: ["SignText"] },
    ]);
    expect(payload.scripts.NurseScript).toEqual([
      { command: "opentext", args: [] },
      { command: "yesorno", args: [] },
      { command: "iffalse", args: [".Done"] },
      { command: "special", args: ["HealParty"] },
      { command: "closetext", args: [] },
      { command: "end", args: [] },
    ]);
    expect(payload.scripts[".Done@NurseScript"]).toEqual([
      { command: "closetext", args: [] },
      { command: "end", args: [] },
    ]);
    expect(standardScriptsStoryEventPayload(payload).StandardScripts.StdScripts).toEqual([
      { command: "add_stdscript", args: ["SignScript"] },
      { command: "add_stdscript", args: ["NurseScript"] },
    ]);
  });

  it("rejects a standard-script table entry without a command body", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "standard-scripts-"));
    const asmSource = path.join(tmpDir, "std_scripts.asm");
    fs.writeFileSync(
      asmSource,
      `StdScripts::
\tadd_stdscript MissingScript
`,
      "utf8",
    );

    expect(() => parseStandardScriptsFile(asmSource)).toThrow(
      "Standard script pointer MissingScript has no parsed command body",
    );
  });

  it("rejects missing and duplicate canonical shared-script sources", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "standard-shared-scripts-"));
    const asmSource = path.join(tmpDir, "std_scripts.asm");
    const duplicateSource = path.join(tmpDir, "shared.asm");
    const missingSource = path.join(tmpDir, "missing.asm");
    fs.writeFileSync(
      asmSource,
      `StdScripts::
\tadd_stdscript SharedScript

SharedScript:
\tend
`,
      "utf8",
    );
    fs.writeFileSync(
      duplicateSource,
      `SharedScript:
\tret
`,
      "utf8",
    );

    expect(() => parseStandardScriptsFile(asmSource, [missingSource])).toThrow(
      `Required shared script source is missing: ${missingSource}`,
    );
    expect(() => parseStandardScriptsFile(asmSource, [duplicateSource])).toThrow(
      `Shared script label SharedScript from ${duplicateSource} duplicates ${asmSource}`,
    );
  });

  it("exports the exact Bug Contest timeout global root and reachable standard-script handoff", () => {
    const root = path.resolve(__dirname, "../../../../vendor/pokecrystal");
    const contestSource = path.join(root, "engine/events/bug_contest/contest.asm");
    const payload = parseStandardScriptsFile(
      path.join(root, "engine/events/std_scripts.asm"),
      [
        path.join(root, "engine/events/overworld.asm"),
        path.join(root, "engine/events/treemons.asm"),
        path.join(root, "engine/events/misc_scripts.asm"),
        path.join(root, "data/wild/treemon_maps.asm"),
        path.join(root, "data/wild/treemons.asm"),
      ],
      [
        {
          filePath: contestSource,
          roots: ["BugCatchingContestOverScript"],
          reachableLabels: [
            "BugCatchingContestOverScript",
            "BugCatchingContestReturnToGateScript",
            "BugCatchingContestTimeUpText",
          ],
          standardTargets: ["BugContestResultsWarpScript"],
        },
      ],
    );

    expect(payload.globalScriptRoots).toEqual(["BugCatchingContestOverScript"]);
    expect(payload.scripts.BugCatchingContestOverScript).toEqual([
      { command: "playsound", args: ["SFX_ELEVATOR_END"] },
      { command: "opentext", args: [] },
      { command: "writetext", args: ["BugCatchingContestTimeUpText"] },
      { command: "waitbutton", args: [] },
      { command: "sjump", args: ["BugCatchingContestReturnToGateScript"] },
    ]);
    expect(payload.scripts.BugCatchingContestReturnToGateScript).toEqual([
      { command: "closetext", args: [] },
      { command: "jumpstd", args: ["BugContestResultsWarpScript"] },
    ]);
    expect(payload.scripts.BugCatchingContestTimeUpText).toEqual([
      { command: "text_far", args: ["_BugCatchingContestTimeUpText"] },
      { command: "text_end", args: [] },
    ]);
    expect(payload.scripts.BugCatchingContestBattleScript).toBeUndefined();
    expect(payload.scripts.BugCatchingContestOutOfBallsScript).toBeUndefined();
    expect(payload.order).toContain("BugContestResultsWarpScript");
    expect(
      standardScriptsStoryEventPayload(payload).StandardScripts.BugContestResultsWarpScript,
    ).toEqual(payload.scripts.BugContestResultsWarpScript);
    expect(
      standardScriptsStoryEventPayload(payload).StandardScripts.GlobalScriptRoots,
    ).toEqual(["BugCatchingContestOverScript"]);
  });

  it("fails closed when a required global root source or exact standard handoff changes", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "global-script-roots-"));
    const standardSource = path.join(tmpDir, "std_scripts.asm");
    const contestSource = path.join(tmpDir, "contest.asm");
    const missingSource = path.join(tmpDir, "missing-contest.asm");
    fs.writeFileSync(
      standardSource,
      `StdScripts::
\tadd_stdscript BugContestResultsWarpScript
\tadd_stdscript BugContestResultsScript

BugContestResultsWarpScript:
\tend

BugContestResultsScript:
\tend
`,
      "utf8",
    );
    const parseContest = (filePath = contestSource) =>
      parseStandardScriptsFile(standardSource, [], [
        {
          filePath,
          roots: ["BugCatchingContestOverScript"],
          reachableLabels: [
            "BugCatchingContestOverScript",
            "BugCatchingContestReturnToGateScript",
          ],
          standardTargets: ["BugContestResultsWarpScript"],
        },
      ]);

    expect(() => parseContest(missingSource)).toThrow(
      `Required global script source is missing: ${missingSource}`,
    );

    fs.writeFileSync(
      contestSource,
      `BugCatchingContestOverScript:
\tsjump BugCatchingContestReturnToGateScript

BugCatchingContestReturnToGateScript:
\tjumpstd BugContestResultsWarpScript
`,
      "utf8",
    );
    expect(() => parseContest()).toThrow(
      `Required global script root BugCatchingContestOverScript must be declared with :: in ${contestSource}`,
    );

    fs.writeFileSync(
      contestSource,
      `BugCatchingContestOverScript::

BugCatchingContestReturnToGateScript:
\tjumpstd BugContestResultsWarpScript
`,
      "utf8",
    );
    expect(() => parseContest()).toThrow(
      `Required global script root BugCatchingContestOverScript has no command body in ${contestSource}`,
    );

    fs.writeFileSync(
      contestSource,
      `BugCatchingContestOverScript::
\tsjump MissingBugContestReturnScript
`,
      "utf8",
    );
    expect(() => parseContest()).toThrow(
      "Global script BugCatchingContestOverScript command 0 sjump targets missing script MissingBugContestReturnScript",
    );

    fs.writeFileSync(
      contestSource,
      `BugCatchingContestOverScript::
\twritetext MissingBugContestTimeUpText
\tsjump BugCatchingContestReturnToGateScript

BugCatchingContestReturnToGateScript:
\tjumpstd BugContestResultsWarpScript
`,
      "utf8",
    );
    expect(() => parseContest()).toThrow(
      "Global script BugCatchingContestOverScript command 0 writetext targets missing local body MissingBugContestTimeUpText",
    );

    fs.writeFileSync(
      contestSource,
      `BugCatchingContestOverScript::
\tsjump BugCatchingContestReturnToGateScript

BugCatchingContestReturnToGateScript:
\tjumpstd BugContestResultsScript
`,
      "utf8",
    );
    expect(() => parseContest()).toThrow(
      `Global script source ${contestSource} must target exactly [BugContestResultsWarpScript] through jumpstd, found [BugContestResultsScript]`,
    );
  });

  it("rejects duplicate global root declarations and duplicate reachable bodies", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "global-script-duplicates-"));
    const standardSource = path.join(tmpDir, "std_scripts.asm");
    const firstSource = path.join(tmpDir, "first.asm");
    const secondSource = path.join(tmpDir, "second.asm");
    fs.writeFileSync(
      standardSource,
      `StdScripts::
\tadd_stdscript BugContestResultsWarpScript

BugContestResultsWarpScript:
\tend
`,
      "utf8",
    );
    fs.writeFileSync(
      firstSource,
      `BugCatchingContestOverScript::
\tsjump SharedReturnScript

SharedReturnScript:
\tjumpstd BugContestResultsWarpScript
`,
      "utf8",
    );
    fs.writeFileSync(
      secondSource,
      `SecondContestRoot::
\tsjump SharedReturnScript

SharedReturnScript:
\tjumpstd BugContestResultsWarpScript
`,
      "utf8",
    );
    const first = {
      filePath: firstSource,
      roots: ["BugCatchingContestOverScript"],
      reachableLabels: ["BugCatchingContestOverScript", "SharedReturnScript"],
      standardTargets: ["BugContestResultsWarpScript"],
    };

    expect(() => parseStandardScriptsFile(standardSource, [], [first, first])).toThrow(
      "Global script root BugCatchingContestOverScript is declared more than once",
    );
    expect(() =>
      parseStandardScriptsFile(standardSource, [], [
        first,
        {
          filePath: secondSource,
          roots: ["SecondContestRoot"],
          reachableLabels: ["SecondContestRoot", "SharedReturnScript"],
          standardTargets: ["BugContestResultsWarpScript"],
        },
      ]),
    ).toThrow(`Global script label SharedReturnScript from ${secondSource} duplicates ${firstSource}`);
  });

  it("parses every authoritative Crystal standard-script pointer", () => {
    const source = path.resolve(
      __dirname,
      "../../../../vendor/pokecrystal/engine/events/std_scripts.asm",
    );
    const sharedOverworldSource = path.resolve(
      __dirname,
      "../../../../vendor/pokecrystal/engine/events/overworld.asm",
    );
    const sharedTreeMonSource = path.resolve(
      __dirname,
      "../../../../vendor/pokecrystal/engine/events/treemons.asm",
    );
    const sharedMiscScriptsSource = path.resolve(
      __dirname,
      "../../../../vendor/pokecrystal/engine/events/misc_scripts.asm",
    );
    const sharedTreeMonMapsSource = path.resolve(
      __dirname,
      "../../../../vendor/pokecrystal/data/wild/treemon_maps.asm",
    );
    const sharedTreeMonsSource = path.resolve(
      __dirname,
      "../../../../vendor/pokecrystal/data/wild/treemons.asm",
    );
    const payload = parseStandardScriptsFile(source, [
      sharedOverworldSource,
      sharedTreeMonSource,
      sharedMiscScriptsSource,
      sharedTreeMonMapsSource,
      sharedTreeMonsSource,
    ]);

    expect(payload.order).toHaveLength(52);
    expect(payload.order[0]).toBe("PokecenterNurseScript");
    expect(payload.order.at(-1)).toBe("HappinessCheckScript");
    expect(Object.keys(payload.scripts)).toEqual(
      expect.arrayContaining([
        "PokecenterNurseScript",
        "DifficultBookshelfScript",
        "GameCornerCoinVendorScript",
        "HappinessCheckScript",
      ]),
    );
    expect(payload.scripts.DifficultBookshelfScript).toEqual([
      { command: "farjumptext", args: ["DifficultBookshelfText"] },
    ]);
    expect(payload.scripts.AskStrengthScript).toEqual(
      expect.arrayContaining([
        { command: "callasm", args: ["TryStrengthOW"] },
        { command: "iffalse", args: [".AskStrength"] },
      ]),
    );
    expect(payload.scripts[".AskStrength@AskStrengthScript"]).toEqual(
      expect.arrayContaining([
        { command: "iftrue", args: ["Script_UsedStrength"] },
      ]),
    );
    expect(payload.scripts.AskRockSmashScript).toEqual(
      expect.arrayContaining([
        { command: "callasm", args: ["HasRockSmash"] },
        { command: "iftrue", args: ["RockSmashScript"] },
      ]),
    );
    expect(payload.scripts.TryStrengthOW).toBeDefined();
    expect(payload.scripts.HasRockSmash).toBeDefined();
    expect(payload.scripts.RockMonEncounter).toEqual(
      expect.arrayContaining([
        { command: "ld", args: ["hl", "RockMonMaps"] },
        { command: "call", args: ["GetTreeMonSet"] },
        { command: "call", args: ["SelectTreeMon"] },
      ]),
    );
    expect(payload.scripts.RockMonMaps).toBeDefined();
    expect(payload.scripts.TreeMonSet_Rock).toBeDefined();
    expect(payload.scripts.Script_AbortBugContest).toEqual([
      { command: "checkflag", args: ["ENGINE_BUG_CONTEST_TIMER"] },
      { command: "iffalse", args: [".finish"] },
      { command: "setflag", args: ["ENGINE_DAILY_BUG_CONTEST"] },
      { command: "special", args: ["ContestReturnMons"] },
      { command: "end", args: [] },
    ]);
  });

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

  it("keeps Azalea's local post-battle script distinct from its global movement", () => {
    const source = path.resolve(
      __dirname,
      "../../../../vendor/pokecrystal/maps/AzaleaTown.asm",
    );

    const scripts = parseAsmFile(source);

    expect(scripts.AzaleaTownRivalBattleScript.at(-1)).toEqual({
      command: "sjump",
      args: [".AfterBattle"],
    });
    expect(scripts.AzaleaTownRivalBattleScript).not.toContainEqual({
      command: "playmusic",
      args: ["MUSIC_RIVAL_AFTER"],
    });
    expect(scripts[".AfterBattle@AzaleaTownRivalBattleScript"]).toEqual(
      expect.arrayContaining([
        {
          command: "applymovement",
          args: ["AZALEATOWN_RIVAL", "AzaleaTownRivalBattleExitMovement"],
        },
      ]),
    );
    expect(scripts.AzaleaTownRivalBattleExitMovement).toEqual([
      { command: "step", args: ["LEFT"] },
      { command: "step", args: ["LEFT"] },
      { command: "step", args: ["LEFT"] },
      { command: "step_end", args: [] },
    ]);
  });

  it("does not synthesize script fallthrough after a far-text body", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "phone.asm");
    fs.writeFileSync(
      asmSource,
      `WrongNumber:
\tdba .script
.script
\twritetext .PhoneWrongNumberText
\tend
.PhoneWrongNumberText:
\ttext_far _PhoneWrongNumberText
\ttext_end

Script_ReceivePhoneCall:
\twaitbutton
\tend
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts[".PhoneWrongNumberText@WrongNumber"]).toEqual([
      { command: "text_far", args: ["_PhoneWrongNumberText"] },
      { command: "text_end", args: [] },
    ]);
  });

  it("distinguishes unconditional and conditional CPU returns", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "phone.asm");
    fs.writeFileSync(
      asmSource,
      `UnconditionalCpuRoutine:
\txor a
\tret

NextCpuRoutine:
\tscf
\tret

ConditionalCpuRoutine:
\tret z

ConditionalFallthrough:
\tscf
\tret

UnconditionalJump:
\tjp NextCpuRoutine

ConditionalJump:
\tjr nz, ConditionalJumpTarget

ConditionalJumpFallthrough:
\txor a
\tret

ConditionalJumpTarget:
\tscf
\tret
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts.UnconditionalCpuRoutine).toEqual([
      { command: "xor", args: ["a"] },
      { command: "ret", args: [] },
    ]);
    expect(scripts.ConditionalCpuRoutine).toEqual([
      { command: "ret", args: ["z"] },
      { command: "jp", args: ["ConditionalFallthrough"] },
    ]);
    expect(scripts.UnconditionalJump).toEqual([
      { command: "jp", args: ["NextCpuRoutine"] },
    ]);
    expect(scripts.ConditionalJump).toEqual([
      { command: "jr", args: ["nz", "ConditionalJumpTarget"] },
      { command: "jp", args: ["ConditionalJumpFallthrough"] },
    ]);
  });

  it("materializes CPU label fallthroughs as CPU jumps, never script jumps", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "overworld.asm");
    fs.writeFileSync(
      asmSource,
      `CheckPartyMove:
\tld e, 0
.loop
\tld b, NUM_MOVES
.check
\tld a, [hli]
\tcp d
\tjr z, .yes
\tdec b
\tjr nz, .check
.next
\tinc e
\tjr .loop
.yes
\txor a
\tret
`,
      "utf8",
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts[".loop@CheckPartyMove"]).toEqual([
      { command: "ld", args: ["b", "NUM_MOVES"] },
      { command: "jp", args: [".check@CheckPartyMove"] },
    ]);
    expect(scripts[".check@CheckPartyMove"]).toEqual([
      { command: "ld", args: ["a", "[hli]"] },
      { command: "cp", args: ["d"] },
      { command: "jr", args: ["z", ".yes"] },
      { command: "dec", args: ["b"] },
      { command: "jr", args: ["nz", ".check"] },
      { command: "jp", args: [".next@CheckPartyMove"] },
    ]);
    expect(
      Object.entries(scripts)
        .filter(([label]) => label.endsWith("@CheckPartyMove"))
        .flatMap(([, body]) => body)
        .filter((command) => command.command === "sjump"),
    ).toEqual([]);

    const canonical = parseAsmFile(
      path.resolve(
        __dirname,
        "../../../../vendor/pokecrystal/engine/events/overworld.asm",
      ),
    );
    expect(canonical[".check@CheckPartyMove"].at(-1)).toEqual({
      command: "jp",
      args: [".next@CheckPartyMove"],
    });
    expect(
      Object.entries(canonical)
        .filter(([label]) =>
          label === "CheckPartyMove" || label.endsWith("@CheckPartyMove"),
        )
        .flatMap(([, body]) => body)
        .some((command) => command.command === "sjump"),
    ).toBe(false);
  });

  it("does not synthesize script fallthrough after source-terminal or map-data macros", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "terminal_macros.asm");
    fs.writeFileSync(
      asmSource,
      `DecorationScript:
\tdescribedecoration DECODESC_CONSOLE

FruitTreeScript:
\tfruittree FRUITTREE_ROUTE_29

ItemBallData:
\titemball POTION

HiddenItemData:
\thiddenitem ELIXER, EVENT_HIDDEN_ELIXER

ConditionalEventData:
\tconditional_event EVENT_POSTER, .Script

.Script:
\tend

MemJumpScript:
\tmemjump wQueuedScriptBank

StopAndJumpScript:
\tstopandsjump StopAndJumpTarget

StopAndJumpTarget:
\tend

ReloadEndScript:
\treloadend MAPSETUP_CONNECTION

EndAllScript:
\tendall

HallOfFameScript:
\thalloffame

CreditsScript:
\tcredits

ConditionalTrainerGuard:
\tendifjustbattled

ConditionalTrainerFallthrough:
\topentext
\tend
`,
      "utf8",
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts.DecorationScript).toEqual([
      { command: "describedecoration", args: ["DECODESC_CONSOLE"] },
    ]);
    expect(scripts.FruitTreeScript).toEqual([
      { command: "fruittree", args: ["FRUITTREE_ROUTE_29"] },
    ]);
    expect(scripts.ItemBallData).toEqual([
      { command: "itemball", args: ["POTION"] },
    ]);
    expect(scripts.HiddenItemData).toEqual([
      { command: "hiddenitem", args: ["ELIXER", "EVENT_HIDDEN_ELIXER"] },
    ]);
    expect(scripts.ConditionalEventData).toEqual([
      { command: "conditional_event", args: ["EVENT_POSTER", ".Script"] },
    ]);
    expect(scripts.MemJumpScript).toEqual([
      { command: "memjump", args: ["wQueuedScriptBank"] },
    ]);
    expect(scripts.StopAndJumpScript).toEqual([
      { command: "stopandsjump", args: ["StopAndJumpTarget"] },
    ]);
    expect(scripts.ReloadEndScript).toEqual([
      { command: "reloadend", args: ["MAPSETUP_CONNECTION"] },
    ]);
    expect(scripts.EndAllScript).toEqual([{ command: "endall", args: [] }]);
    expect(scripts.HallOfFameScript).toEqual([{ command: "halloffame", args: [] }]);
    expect(scripts.CreditsScript).toEqual([{ command: "credits", args: [] }]);
    expect(scripts.ConditionalTrainerGuard).toEqual([
      { command: "endifjustbattled", args: [] },
      { command: "sjump", args: ["ConditionalTrainerFallthrough"] },
    ]);
  });

  it("keeps every authoritative unconditional script transfer free of synthesized fallthrough", () => {
    const root = path.resolve(__dirname, "../../../../vendor/pokecrystal");
    const sourcePaths = [
      ...fs
        .readdirSync(path.join(root, "maps"))
        .filter((entry) => entry.endsWith(".asm"))
        .sort()
        .map((entry) => path.join(root, "maps", entry)),
      path.join(root, "engine/events/std_scripts.asm"),
      path.join(root, "engine/events/overworld.asm"),
      path.join(root, "engine/events/treemons.asm"),
      path.join(root, "engine/events/misc_scripts.asm"),
      path.join(root, "engine/events/whiteout.asm"),
      path.join(root, "engine/overworld/events.asm"),
    ];
    const issues: string[] = [];
    for (const sourcePath of sourcePaths) {
      const scripts = parseAsmFile(sourcePath);
      for (const [label, body] of Object.entries(scripts)) {
        for (let commandIndex = 0; commandIndex + 1 < body.length; commandIndex += 1) {
          if (
            isUnconditionalScriptTransferCommand(body[commandIndex].command) &&
            body[commandIndex + 1].command === "sjump"
          ) {
            issues.push(
              `${path.relative(root, sourcePath)}:${label}:${commandIndex} ${body[commandIndex].command} acquired sjump ${String(body[commandIndex + 1].args)}`,
            );
          }
        }
      }
    }

    expect(issues).toEqual([]);
    const overworldEvents = parseAsmFile(path.join(root, "engine/overworld/events.asm"));
    expect(overworldEvents.EdgeWarpScript).toEqual([
      { command: "reloadend", args: ["MAPSETUP_CONNECTION"] },
    ]);
    expect(overworldEvents.ChangeDirectionScript?.[0]?.command).toBe("deactivatefacing");
  });

  it("keeps PlayersHouse2F decoration and conditional-event bodies source exact", () => {
    const source = path.resolve(
      __dirname,
      "../../../../vendor/pokecrystal/maps/PlayersHouse2F.asm",
    );

    const scripts = parseAsmFile(source);

    expect(scripts.PlayersHouseGameConsoleScript).toEqual([
      { command: "describedecoration", args: ["DECODESC_CONSOLE"] },
    ]);
    expect(scripts.PlayersHousePosterScript).toEqual([
      {
        command: "conditional_event",
        args: ["EVENT_PLAYERS_ROOM_POSTER", ".Script"],
      },
    ]);
    expect(scripts[".Script@PlayersHousePosterScript"]).toEqual([
      { command: "describedecoration", args: ["DECODESC_POSTER"] },
    ]);
  });

  it("exports every authoritative map control edge to an executable script body", () => {
    const mapsRoot = path.resolve(__dirname, "../../../../vendor/pokecrystal/maps");
    const parsedMaps = fs
      .readdirSync(mapsRoot)
      .filter((entry) => entry.endsWith(".asm"))
      .sort()
      .map((entry) => ({ entry, scripts: parseAsmFile(path.join(mapsRoot, entry)) }));
    const exactBodies = new Map<string, Array<{ command: string; args: unknown }>>();
    for (const { scripts } of parsedMaps) {
      for (const [label, body] of Object.entries(scripts)) {
        if (!exactBodies.has(label)) {
          exactBodies.set(label, body);
        }
      }
    }
    const compareCommands = new Set(["ifequal", "ifnotequal", "ifgreater", "ifless"]);
    const targetCommands = new Set([
      ...compareCommands,
      "iftrue",
      "iffalse",
      "sjump",
      "jump",
      "farsjump",
      "scall",
      "farscall",
      "sdefer",
    ]);
    const issues: string[] = [];
    for (const { entry, scripts } of parsedMaps) {
      for (const [sourceLabel, body] of Object.entries(scripts)) {
        for (const [commandIndex, command] of body.entries()) {
          if (!targetCommands.has(command.command)) {
            continue;
          }
          const args = Array.isArray(command.args) ? command.args.map(String) : [];
          const target = args[compareCommands.has(command.command) ? 1 : 0];
          const parent = sourceLabel.includes("@")
            ? sourceLabel.slice(sourceLabel.lastIndexOf("@") + 1)
            : sourceLabel;
          const localTarget = target?.startsWith(".") ? `${target}@${parent}` : undefined;
          const targetBody =
            scripts[target] ??
            (localTarget ? scripts[localTarget] : undefined) ??
            exactBodies.get(target);
          const firstCommand = targetBody?.[0]?.command;
          if (!targetBody?.length || !firstCommand) {
            issues.push(`${entry}:${sourceLabel}:${commandIndex} ${command.command} -> ${target} is missing`);
          } else if (!isExecutableScriptControlTargetBody(targetBody)) {
            issues.push(
              `${entry}:${sourceLabel}:${commandIndex} ${command.command} -> ${target} begins ${firstCommand}`,
            );
          }
        }
      }
    }

    expect(issues).toEqual([]);
  });

  it("exports every authoritative global control edge to a matching executable body", () => {
    const root = path.resolve(__dirname, "../../../../vendor/pokecrystal");
    const payload = parseStandardScriptsFile(
      path.join(root, "engine/events/std_scripts.asm"),
      [
        path.join(root, "engine/events/overworld.asm"),
        path.join(root, "engine/events/treemons.asm"),
        path.join(root, "engine/events/misc_scripts.asm"),
        path.join(root, "data/wild/treemon_maps.asm"),
        path.join(root, "data/wild/treemons.asm"),
      ],
    );
    const compareCommands = new Set(["ifequal", "ifnotequal", "ifgreater", "ifless"]);
    const targetCommands = new Set([
      ...compareCommands,
      "iftrue",
      "iffalse",
      "sjump",
      "jump",
      "farsjump",
      "scall",
      "farscall",
      "sdefer",
    ]);
    const issues: string[] = [];
    for (const [sourceLabel, body] of Object.entries(payload.scripts)) {
      for (const [commandIndex, command] of body.entries()) {
        if (!targetCommands.has(command.command)) {
          continue;
        }
        const args = Array.isArray(command.args) ? command.args.map(String) : [];
        const target = args[compareCommands.has(command.command) ? 1 : 0];
        const parent = sourceLabel.includes("@")
          ? sourceLabel.slice(sourceLabel.lastIndexOf("@") + 1)
          : sourceLabel;
        const localTarget = target?.startsWith(".") ? `${target}@${parent}` : undefined;
        const resolvedTarget =
          payload.scripts[target] ?? (localTarget ? payload.scripts[localTarget] : undefined);
        if (!resolvedTarget?.length) {
          issues.push(`${sourceLabel}:${commandIndex} ${command.command} -> ${target} is missing`);
        } else if (!isExecutableScriptControlEdge(body, resolvedTarget)) {
          issues.push(
            `${sourceLabel}:${commandIndex} ${command.command} -> ${target} begins ${resolvedTarget[0]?.command}`,
          );
        }
      }
    }

    expect(issues).toEqual([]);
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

  it("materializes local script fallthrough into the next parent script label", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "PlayersHouse1F.asm");
    fs.writeFileSync(
      asmSource,
      `MeetMomRightScript:
\tcheckevent EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1
\tiffalse .OnRight
\tapplymovement PLAYERSHOUSE1F_MOM1, MomTurnsTowardPlayerMovement
\tsjump MeetMomScript

.OnRight:
\tapplymovement PLAYERSHOUSE1F_MOM1, MomWalksToPlayerMovement
MeetMomScript:
\topentext
\twritetext ElmsLookingForYouText
\twaitbutton
\tclosetext
\tend
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts[".OnRight@MeetMomRightScript"].at(-1)).toEqual({
      command: "sjump",
      args: ["MeetMomScript"],
    });
  });

  it("exports warpfacing in canonical pack order", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "Route29.asm");
    fs.writeFileSync(
      asmSource,
      `Route29WarpScript:
\twarpfacing RIGHT, ROUTE_29, 6, 27
\tend
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts.Route29WarpScript[0]).toEqual({
      command: "warpfacing",
      args: ["ROUTE_29", "6", "27", "RIGHT"],
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

  it("materializes movement data fallthrough through the next movement terminator", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "story-events-"));
    const asmSource = path.join(tmpDir, "BattleTower1F.asm");
    fs.writeFileSync(
      asmSource,
      `MovementData_BattleTower1FWalkToElevator:
\tstep UP
\tstep UP
MovementData_BattleTowerHallwayPlayerEntersBattleRoom:
\tstep UP
\tstep_end
`,
      "utf8"
    );

    const scripts = parseAsmFile(asmSource);

    expect(scripts.MovementData_BattleTower1FWalkToElevator).toEqual([
      { command: "step", args: ["UP"] },
      { command: "step", args: ["UP"] },
      { command: "step", args: ["UP"] },
      { command: "step_end", args: [] },
    ]);
    expect(scripts.MovementData_BattleTowerHallwayPlayerEntersBattleRoom).toEqual([
      { command: "step", args: ["UP"] },
      { command: "step_end", args: [] },
    ]);
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
