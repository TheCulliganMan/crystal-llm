import fs from "fs";
import path from "path";
import { getAssetsRoot } from "@pokecrystal/core/core/paths";
import { exportBattleRewardRules } from "./export-battle-reward-rules";
import { exportPhoneScripts } from "./export-phone-scripts";
import { exportStoryEvents } from "./export-story-events";

describe("overworld postbattle exporter integration", () => {
  it("writes every queued overworld field-script root to the canonical catalog", () => {
    exportStoryEvents();

    const catalog = JSON.parse(
      fs.readFileSync(
        path.join(getAssetsRoot(), "data", "story_events", "StandardScripts.json"),
        "utf8",
      ),
    ) as { StandardScripts: { GlobalScriptRoots: string[] } };
    expect(catalog.StandardScripts.GlobalScriptRoots).toEqual(
      expect.arrayContaining([
        "Script_CutFromMenu",
        "Script_UseFlash",
        "SurfFromMenuScript",
        ".FlyScript@FlyFunction",
        ".UsedDigScript@EscapeRopeOrDig",
        ".UsedEscapeRopeScript@EscapeRopeOrDig",
        ".TeleportScript@TeleportFunction",
        "Script_GotABite",
        "Script_GetOnBike",
        "Script_GetOffBike",
        "Script_ForcedMovement",
      ]),
    );
  });

  it("writes the complete ASM Mom purchase tables and deferred message scripts", () => {
    const rules = exportBattleRewardRules();
    exportPhoneScripts();

    expect(rules.mom_random_items).toHaveLength(5);
    expect(rules.mom_progression_items).toHaveLength(10);
    const momScripts = JSON.parse(
      fs.readFileSync(path.join(getAssetsRoot(), "data", "phone_scripts", "mom_purchase.json"), "utf8"),
    ) as Record<string, unknown>;
    expect(Object.keys(momScripts).sort()).toEqual([
      ".DollScript@Mom_GetScriptPointer",
      ".ItemScript@Mom_GetScriptPointer",
      "MomBoughtWithYourMoneyText",
      "MomFoundADollText",
      "MomFoundAnItemText",
      "MomHiHowAreYouText",
      "MomItsInPCText",
      "MomItsInYourRoomText",
    ]);
  });

  it("writes global player-event roots through the real modular content-pack builder", () => {
    const dataRoot = path.join(getAssetsRoot(), "data");
    const compiled = JSON.parse(
      fs.readFileSync(
        path.join(dataRoot, "content-packs", "core-modular.compiled.json"),
        "utf8",
      ),
    ) as {
      categories: {
        audio: Array<Record<string, unknown>>;
        pokemon_cries: Array<Record<string, unknown>>;
      };
    };
    const cachedAudio = Object.assign({}, ...compiled.categories.audio);
    const cachedPokemonCries = Object.assign({}, ...compiled.categories.pokemon_cries);

    jest.doMock("./export-audio-assets", () => ({
      exportAudioAssets: () => cachedAudio,
      exportPokemonCryMetadataFromAsm: () => cachedPokemonCries,
    }));
    jest.doMock("./export-graphics-assets", () => ({
      exportGraphicsAssets: () => undefined,
    }));
    jest.isolateModules(() => {
      const { exportCoreData } = require("./index") as typeof import("./index");
      exportCoreData();
    });

    const packedPath = path.join(
      dataRoot,
      "content-packs",
      "core-modular",
      "story_events",
      "OverworldEvents.json",
    );
    const catalog = JSON.parse(fs.readFileSync(packedPath, "utf8")) as {
      OverworldEvents: Record<string, Array<{ command: string; args: string[] }>>;
    };
    expect(catalog.OverworldEvents.ChangeDirectionScript).toEqual([
      { command: "deactivatefacing", args: ["3"] },
      { command: "callasm", args: ["EnableWildEncounters"] },
      { command: "end", args: [] },
    ]);
    expect(catalog.OverworldEvents.SeenByTrainerScript).toEqual(
      expect.arrayContaining([
        { command: "callasm", args: ["TrainerWalkToPlayer"] },
        { command: "applymovementlasttalked", args: ["wMovementBuffer"] },
      ]),
    );
    expect(catalog.OverworldEvents.TrainerWalkToPlayer).toEqual(expect.any(Array));
    expect(catalog.OverworldEvents.OverworldHatchEgg).toEqual(expect.any(Array));
  });
});
