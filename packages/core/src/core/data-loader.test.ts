import fs from "fs";
import path from "path";

const REAL_READ_FILE_SYNC = fs.readFileSync;
const REAL_READ_FILE = fs.promises.readFile.bind(fs.promises);

const STORY_EVENTS_FIXTURE = {
  PlayersHouse1F: {
    MeetMomLeftScript: [{ command: "setevent", args: ["EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1"] }],
    MeetMomRightScript: [{ command: "end", args: [] }],
    ".OnRight": [
      {
        command: "applymovement",
        args: ["PLAYERSHOUSE1F_MOM1", "MomWalksToPlayerMovement"],
      },
    ],
    EmptySceneScript: [],
    ScriptAfterEmpty: [{ command: "end", args: [] }],
    ScriptNoEnd: [{ command: "setevent", args: ["EVENT_TEMPORARY_UNTIL_MAP_RELOAD_2"] }],
    TextLabel: [{ command: "text", args: ["Hello"] }],
    ScriptAfterText: [{ command: "end", args: [] }],
    FruitTreeScript: [{ command: "fruittree", args: ["FRUITTREE_ROUTE_29"] }],
    ScriptAfterFruitTree: [{ command: "setevent", args: ["EVENT_TEMPORARY_UNTIL_MAP_RELOAD_3"] }],
    ItemBallScript: [{ command: "itemball", args: ["POTION"] }],
    ScriptAfterItemBall: [{ command: "setevent", args: ["EVENT_TEMPORARY_UNTIL_MAP_RELOAD_4"] }],
    HiddenItemScript: [{ command: "hiddenitem", args: ["POTION", "EVENT_TEST_HIDDEN_ITEM"] }],
    ScriptAfterHiddenItem: [{ command: "setevent", args: ["EVENT_TEMPORARY_UNTIL_MAP_RELOAD_5"] }],
    ScriptBeforeMovementData: [{ command: "closetext", args: [] }],
    PlayerWalksAwayMovement: [
      { command: "step", args: ["LEFT"] },
      { command: "step_end", args: [] },
    ],
    ScriptAfterMovementData: [{ command: "setevent", args: ["EVENT_TEMPORARY_UNTIL_MAP_RELOAD_6"] }],
  },
  OtherMap: {
    ParentScript: [{ command: "end", args: [] }],
    ".Local": [{ command: "setevent", args: ["EVENT_TEMPORARY_UNTIL_MAP_RELOAD_3"] }],
  },
};

const SPECIES_FIXTURE = {
  evolutions: null,
  id: "TOTODILE",
  int_id: 158,
  base_stats: {
    hp: 50,
    attack: 65,
    defense: 64,
    speed: 43,
    special_attack: 44,
    special_defense: 48,
  },
  type1: "WATER",
  type2: "WATER",
  catch_rate: 45,
  base_exp: 66,
  item1: null,
  item2: null,
  gender_ratio: 31,
  unknown1: 0,
  step_cycles_to_hatch: 20,
  unknown2: 0,
  growth_rate: "GROWTH_MEDIUM_SLOW",
  egg_group1: "EGG_MONSTER",
  egg_group2: "EGG_WATER_1",
};

const MOVE_FIXTURE = {
  name: "TACKLE",
  type: "NORMAL",
  power: 35,
  accuracy: 95,
  pp: 35,
  effect: "NORMAL_HIT",
  effect_chance: 0,
};

const ITEM_FIXTURE = {
  name: "POTION",
  description: "Heals 20 HP.",
  effect: "NONE",
  price: 300,
  pocket: "ITEM",
};

const PHONE_SCRIPTS_FIXTURE = {
  ElmPhoneCallerScript: [{ command: "end", args: [] }],
};

describe("DataLoader local label resolution", () => {
  afterEach(() => {
    jest.restoreAllMocks();
    jest.dontMock("./content-packs");
  });

  it("prefers scoped local labels so Mom walks to the player", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(STORY_EVENTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();
    const script = loader.get_script(".OnRight", "MeetMomRightScript");

    expect(script).toEqual(STORY_EVENTS_FIXTURE.PlayersHouse1F[".OnRight"]);
  });

  it("exposes fallthrough successors for sequential scripts", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(STORY_EVENTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();
    const successor = loader.get_script_successor("MeetMomLeftScript");

    expect(successor).toEqual([null, "MeetMomRightScript"]);
  });

  it("does not chain through text labels", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(STORY_EVENTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script("TextLabel")).toBe(null);
    expect(loader.get_script_successor("ScriptNoEnd")).toBe(null);
  });

  it("does not chain through prompt or next text labels", async () => {
    const storyEventsFixture = {
      PlayersHouse1F: {
        ScriptBeforePromptText: [{ command: "closetext", args: [] }],
        PromptTextLabel: [
          { command: "text", args: ["Hello"] },
          { command: "prompt", args: [] },
        ],
        ScriptBeforeNextText: [{ command: "closetext", args: [] }],
        NextTextLabel: [
          { command: "text", args: ["Hello"] },
          { command: "next", args: ["page"] },
          { command: "done", args: [] },
        ],
        ScriptAfterText: [{ command: "end", args: [] }],
      },
    };

    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(storyEventsFixture);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script("PromptTextLabel")).toBe(null);
    expect(loader.get_script("NextTextLabel")).toBe(null);
    expect(loader.get_script_successor("ScriptBeforePromptText")).toBe(null);
    expect(loader.get_script_successor("ScriptBeforeNextText")).toBe(null);
  });

  it("does not chain through non-script data labels", async () => {
    const storyEventsFixture = {
      PlayersHouse1F: {
        ScriptBeforeStringData: [{ command: "closetext", args: [] }],
        StringDataLabel: [{ command: "db", args: ["MAP CARD"] }],
        ScriptBeforeMenuData: [{ command: "closetext", args: [] }],
        MenuHeaderLabel: [
          { command: "db", args: ["MENU_BACKUP_TILES"] },
          { command: "menu_coords", args: ["0", "0", "10", "8"] },
          { command: "dw", args: ["MenuDataLabel"] },
        ],
        MenuDataLabel: [{ command: "db", args: ["STATICMENU_CURSOR"] }],
        ScriptAfterData: [{ command: "end", args: [] }],
      },
    };

    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(storyEventsFixture);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script("StringDataLabel")).toEqual(
      storyEventsFixture.PlayersHouse1F.StringDataLabel
    );
    expect(loader.get_script("MenuHeaderLabel")).toEqual(
      storyEventsFixture.PlayersHouse1F.MenuHeaderLabel
    );
    expect(loader.get_script_successor("ScriptBeforeStringData")).toBe(null);
    expect(loader.get_script_successor("StringDataLabel")).toBe(null);
    expect(loader.get_script_successor("ScriptBeforeMenuData")).toBe(null);
    expect(loader.get_script_successor("MenuHeaderLabel")).toBe(null);
  });

  it("does not treat trailing embedded data as script fallthrough", async () => {
    const storyEventsFixture = {
      PlayersHouse1F: {
        ParentMenuScript: [
          { command: "opentext", args: [] },
          { command: "writetext", args: ["ParentMenuText"] },
          { command: "end", args: [] },
          { command: "db", args: ["MENU_BACKUP_TILES"] },
          { command: "menu_coords", args: ["0", "0", "10", "8"] },
          { command: "dw", args: ["ParentMenuData"] },
        ],
        ScriptAfterParentData: [{ command: "setevent", args: ["EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1"] }],
      },
    };

    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(storyEventsFixture);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script_successor("ParentMenuScript")).toBe(null);
  });

  it("exposes text labels from story events", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(STORY_EVENTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_text("TextLabel")).toBe("Hello");
  });

  it("does not fall through empty scripts", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(STORY_EVENTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script_successor("EmptySceneScript")).toBe(null);
  });

  it("does not fall through fruit tree scripts", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(STORY_EVENTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script_successor("FruitTreeScript")).toBe(null);
  });

  it("does not fall through item ball or hidden item scripts", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(STORY_EVENTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script_successor("ItemBallScript")).toBe(null);
    expect(loader.get_script_successor("HiddenItemScript")).toBe(null);
  });

  it("does not fall through movement data labels", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(STORY_EVENTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script("PlayerWalksAwayMovement")).toEqual(
      STORY_EVENTS_FIXTURE.PlayersHouse1F.PlayerWalksAwayMovement
    );
    expect(loader.get_script_successor("ScriptBeforeMovementData")).toBe(null);
    expect(loader.get_script_successor("PlayerWalksAwayMovement")).toBe(null);
  });

  it("does not fall back to global labels for local scripts", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return JSON.stringify(STORY_EVENTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script(".Local", "MeetMomRightScript")).toBe(null);
    expect(loader.get_script(".Local", "ParentScript")).toEqual(
      STORY_EVENTS_FIXTURE.OtherMap[".Local"]
    );
  });

  it("resolves bundled standard scripts when file existence checks miss but asset listings still expose them", async () => {
    const storyEventsFixture = {
      CherrygrovePokecenter1F: {
        CherrygrovePokecenter1FNurseScript: [
          { command: "jumpstd", args: ["PokecenterNurseScript"] },
        ],
      },
      PokecenterNurseScript: [
        { command: "special", args: ["HealParty"] },
        { command: "end", args: [] },
      ],
    };

    jest.resetModules();
    jest.doMock("./asset-manifest", () => {
      const actual = jest.requireActual("./asset-manifest");
      return {
        __esModule: true,
        ...actual,
        listAssetDir: jest.fn(() => ["story_events.json"]),
      };
    });
    try {
      jest.spyOn(fs, "existsSync").mockImplementation(() => false);
      jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
        if (String(pathLike).endsWith("story_events.json")) {
          return JSON.stringify(storyEventsFixture);
        }
        return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
      });

      const { DataLoader } = await import("./data-loader");
      const loader = new DataLoader();

      expect(loader.get_script("PokecenterNurseScript")).toEqual(
        storyEventsFixture.PokecenterNurseScript
      );
      expect(loader.get_script("CherrygrovePokecenter1FNurseScript")).toEqual(
        storyEventsFixture.CherrygrovePokecenter1F.CherrygrovePokecenter1FNurseScript
      );
    } finally {
      jest.dontMock("./asset-manifest");
    }
  });

  it("prefers map scripts when available for map labels", async () => {
    const storyEventsFixture = {
      PlayersHouse1F: {
        MeetMomScript: [{ command: "special", args: ["SetDayOfWeek"] }],
      },
    };
    const mapFixture = {
      MeetMomScript: [
        { command: "opentext", args: [] },
        { command: "end", args: [] },
      ],
      ".SetDayOfWeek@MeetMomScript": [{ command: "end", args: [] }],
    };

    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const path = String(pathLike);
      return path.endsWith("story_events.json") || path.endsWith("PlayersHouse1F.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const path = String(pathLike);
      if (path.endsWith("story_events.json")) {
        return JSON.stringify(storyEventsFixture);
      }
      if (path.endsWith("PlayersHouse1F.json")) {
        return JSON.stringify(mapFixture);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script("MeetMomScript")).toEqual(mapFixture.MeetMomScript);
    expect(loader.get_script(".SetDayOfWeek", "MeetMomScript")).toEqual(
      mapFixture[".SetDayOfWeek@MeetMomScript"]
    );
  });

  it("throws when a referenced map script JSON is malformed instead of warning and returning null", async () => {
    jest.resetModules();
    const warn = jest.spyOn(console, "warn").mockImplementation(() => {});
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const value = String(pathLike);
      return value.endsWith("story_events.json") || value.endsWith("PlayersHouse1F.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const value = String(pathLike);
      if (value.endsWith("story_events.json")) {
        return JSON.stringify({
          PlayersHouse1F: {
            MeetMomScript: [{ command: "end", args: [] }],
          },
        });
      }
      if (value.endsWith("PlayersHouse1F.json")) {
        return "{not valid json";
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.get_script("MeetMomScript")).toThrow(
      /ASM-backed map scripts are required for PlayersHouse1F/
    );
    expect(warn).not.toHaveBeenCalled();
  });

  it("does not link a local map script through a following text label", async () => {
    const storyEventsFixture = {
      PlayersHouse1F: {
        ParentScript: [{ command: "special", args: ["SetDayOfWeek"] }],
      },
    };
    const mapFixture = {
      ParentScript: [{ command: "iftrue", args: [".Local"] }],
      ".Local@ParentScript": [{ command: "closetext", args: [] }],
      ParentText: [{ command: "text", args: ["Hello"] }],
      ScriptAfterText: [{ command: "end", args: [] }],
    };

    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const path = String(pathLike);
      return path.endsWith("story_events.json") || path.endsWith("PlayersHouse1F.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const path = String(pathLike);
      if (path.endsWith("story_events.json")) {
        return JSON.stringify(storyEventsFixture);
      }
      if (path.endsWith("PlayersHouse1F.json")) {
        return JSON.stringify(mapFixture);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script(".Local", "ParentScript")).toEqual(mapFixture[".Local@ParentScript"]);
    expect(loader.get_script_successor(".Local", "ParentScript")).toBe(null);
  });

  it("does not link a local map script through following movement data", async () => {
    const storyEventsFixture = {
      PlayersHouse1F: {
        ParentScript: [{ command: "special", args: ["SetDayOfWeek"] }],
      },
    };
    const mapFixture = {
      ParentScript: [{ command: "iftrue", args: [".Local"] }],
      ".Local@ParentScript": [{ command: "closetext", args: [] }],
      ".Movement1@ParentScript": [
        { command: "step", args: ["RIGHT"] },
        { command: "step_end", args: [] },
      ],
      ScriptAfterLocalMovement: [{ command: "end", args: [] }],
      PlayerWalksAwayMovement: [
        { command: "step", args: ["LEFT"] },
        { command: "step_end", args: [] },
      ],
      ScriptAfterMovement: [{ command: "end", args: [] }],
    };

    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const path = String(pathLike);
      return path.endsWith("story_events.json") || path.endsWith("PlayersHouse1F.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const path = String(pathLike);
      if (path.endsWith("story_events.json")) {
        return JSON.stringify(storyEventsFixture);
      }
      if (path.endsWith("PlayersHouse1F.json")) {
        return JSON.stringify(mapFixture);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script(".Movement1", "ParentScript")).toEqual(
      mapFixture[".Movement1@ParentScript"]
    );
    expect(loader.get_script_successor(".Local", "ParentScript")).toBe(null);
    expect(loader.get_script_successor(".Movement1", "ParentScript")).toBe(null);
    expect(loader.get_script_successor("PlayerWalksAwayMovement", "ParentScript")).toBe(null);
  });

  it("loads phone scripts for direct script lookup", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return (
        pathValue.endsWith("story_events.json") ||
        pathValue.endsWith(`${path.sep}phone_scripts`) ||
        pathValue.endsWith(`${path.sep}phone_scripts${path.sep}elm.json`)
      );
    });
    jest
      .spyOn(fs, "readdirSync")
      .mockImplementation(((pathLike: fs.PathLike) => {
        const pathValue = String(pathLike);
        if (pathValue.endsWith(`${path.sep}phone_scripts`)) {
          return [
            {
              name: "elm.json",
              isFile: () => true,
            },
          ] as unknown as fs.Dirent[];
        }
        return [] as unknown as fs.Dirent[];
      }) as unknown as typeof fs.readdirSync);
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(`${path.sep}phone_scripts${path.sep}elm.json`)) {
        return JSON.stringify(PHONE_SCRIPTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script("ElmPhoneCallerScript")).toEqual(
      PHONE_SCRIPTS_FIXTURE.ElmPhoneCallerScript
    );
  });

  it("handles phone script directories without dirent metadata", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return (
        pathValue.endsWith("story_events.json") ||
        pathValue.endsWith(`${path.sep}phone_scripts`) ||
        pathValue.endsWith(`${path.sep}phone_scripts${path.sep}elm.json`)
      );
    });
    jest
      .spyOn(fs, "readdirSync")
      .mockImplementation(((pathLike: fs.PathLike) => {
        const pathValue = String(pathLike);
        if (pathValue.endsWith(`${path.sep}phone_scripts`)) {
          return ["elm.json"] as unknown as string[];
        }
        return [] as unknown as string[];
      }) as unknown as typeof fs.readdirSync);
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(`${path.sep}phone_scripts${path.sep}elm.json`)) {
        return JSON.stringify(PHONE_SCRIPTS_FIXTURE);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script("ElmPhoneCallerScript")).toEqual(
      PHONE_SCRIPTS_FIXTURE.ElmPhoneCallerScript
    );
  });

  it("throws when a phone script JSON entry is malformed instead of silently omitting ASM-backed phone labels", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return (
        pathValue.endsWith("story_events.json") ||
        pathValue.endsWith(`${path.sep}phone_scripts`) ||
        pathValue.endsWith(`${path.sep}phone_scripts${path.sep}elm.json`)
      );
    });
    jest
      .spyOn(fs, "readdirSync")
      .mockImplementation(((pathLike: fs.PathLike) => {
        const pathValue = String(pathLike);
        if (pathValue.endsWith(`${path.sep}phone_scripts`)) {
          return [
            {
              name: "elm.json",
              isFile: () => true,
            },
          ] as unknown as fs.Dirent[];
        }
        return [] as unknown as fs.Dirent[];
      }) as unknown as typeof fs.readdirSync);
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(`${path.sep}phone_scripts${path.sep}elm.json`)) {
        return JSON.stringify({ ElmPhoneCallerScript: "not an array" });
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.get_script("ElmPhoneCallerScript")).toThrow(
      "ASM-backed phone scripts are required;"
    );
  });

  it("falls back to underscore-prefixed ASM text labels", async () => {
    jest.resetModules();
    jest.spyOn(fs, "existsSync").mockReturnValue(false);
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const asmGet = jest.fn((label: string) => {
      if (label === "_CaughtAskNicknameText") {
        return "Give a nickname to the newly caught POKEMON?";
      }
      return "";
    });
    jest.doMock("./asm-text-loader", () => ({
      asmTextLoader: { get: asmGet },
    }));

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_text("CaughtAskNicknameText")).toBe(
      "Give a nickname to the newly caught POKEMON?"
    );
    expect(asmGet).toHaveBeenCalledWith("_CaughtAskNicknameText");
  });
});


describe("DataLoader warp loading", () => {
  it("preserves Cherrygrove Pokecenter door warp ordering and coordinates", async () => {
    jest.resetModules();

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();
    loader.ensure_map_scripts("CherrygrovePokecenter1F");

    expect(loader.map_events.get("CherrygrovePokecenter1F")?.warps).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          index: 1,
          x: 3,
          y: 7,
          target_map_constant: "CHERRYGROVE_CITY",
          target_warp_id: 2,
        }),
        expect.objectContaining({
          index: 2,
          x: 4,
          y: 7,
          target_map_constant: "CHERRYGROVE_CITY",
          target_warp_id: 2,
        }),
        expect.objectContaining({
          index: 3,
          x: 0,
          y: 7,
          target_map_constant: "POKECENTER_2F",
          target_warp_id: 1,
        }),
      ])
    );
  });

  it("loads Route29 and its connection targets from exported modular map data", async () => {
    jest.resetModules();

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();
    loader.load_map_attributes();
    loader.load_map_dimensions();
    loader.ensure_map_scripts("Route29");

    const route29Attributes = loader.map_attributes.get("Route29");
    expect(route29Attributes?.connections).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ direction: "north", target_map: "Route46" }),
        expect.objectContaining({ direction: "west", target_map: "CherrygroveCity" }),
        expect.objectContaining({ direction: "east", target_map: "NewBarkTown" }),
      ])
    );
    expect(loader.map_events.get("Route29")?.warps).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          target_map_constant: "ROUTE_29_ROUTE_46_GATE",
          target_map: "Route29Route46Gate",
        }),
      ])
    );

    for (const targetMap of ["Route46", "CherrygroveCity", "NewBarkTown"]) {
      const targetAttributes = loader.map_attributes.get(targetMap);
      expect(targetAttributes).toBeDefined();
      expect(loader.map_dimensions.get(String(targetAttributes?.map_constant))).toEqual(
        expect.objectContaining({
          width: expect.any(Number),
          height: expect.any(Number),
        })
      );
      loader.ensure_map_scripts(targetMap);
      expect(loader.map_events.get(targetMap)).toEqual(
        expect.objectContaining({
          warps: expect.any(Array),
        })
      );
    }
  });
});

describe("DataLoader exported dialogue and script loading", () => {
  it("loads sampled map dialogue text from exported modular map payloads", async () => {
    jest.resetModules();

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_text("Route29YoungsterText")).toContain("Yo. How are your");
    expect(loader.get_text("NewBarkTownSignText")).toContain("NEW BARK TOWN");
    expect(loader.get_text("GuideGentIntroText")).toContain("You're a rookie\ntrainer");
    expect(loader.get_text("VioletCityLassText")).toContain("SPROUT TOWER");
    expect(loader.get_text("GoldenrodGameCornerPokefanM2Text")).toContain("I couldn't win at");
  });

  it("loads sampled map scripts, local labels, event scripts, and phone scripts after export", async () => {
    jest.resetModules();

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script("Route29YoungsterScript")).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ command: "jumptextfaceplayer", args: ["Route29YoungsterText"] }),
      ])
    );
    expect(loader.get_script(".HaveMapCard", "CherrygroveTeacherScript")).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ command: "writetext", args: ["CherrygroveTeacherText_HaveMapCard"] }),
      ])
    );
    loader.ensure_map_scripts("Route29");
    expect(loader.map_events.get("Route29")?.bg_events).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ script: "Route29Sign1" }),
        expect.objectContaining({ script: "Route29Sign2" }),
      ])
    );
    expect(loader.get_script("ElmPhoneCallerScript")).toEqual(expect.any(Array));
  });

  it("links Route 36 Sudowoodo's local fight label to the watered-tree script", async () => {
    jest.resetModules();

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script(".Fight", "SudowoodoScript")).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ command: "yesorno" }),
        expect.objectContaining({ command: "closetext" }),
      ])
    );
    expect(loader.get_script_successor(".Fight", "SudowoodoScript")).toEqual([
      null,
      "WateredWeirdTreeScript",
    ]);
  });
});

describe("DataLoader species loading", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("loads species from array payloads keyed by id", async () => {
    jest.resetModules();
    jest.spyOn(fs.promises, "readFile").mockResolvedValue(JSON.stringify([SPECIES_FIXTURE]));

    const { loadSpecies, getSpecies, speciesMap } = await import("./data-loader");
    await loadSpecies();

    expect(getSpecies("totodile")?.id).toBe("TOTODILE");
    expect(speciesMap.get("TOTODILE")?.id).toBe("TOTODILE");
  });

  it("loads species from object payloads keyed by name", async () => {
    jest.resetModules();
    jest
      .spyOn(fs.promises, "readFile")
      .mockResolvedValue(JSON.stringify({ TOTODILE: SPECIES_FIXTURE }));

    const { loadSpecies, getSpecies, speciesMap } = await import("./data-loader");
    await loadSpecies();

    expect(getSpecies("TOTODILE")?.id).toBe("TOTODILE");
    expect(speciesMap.get("TOTODILE")?.id).toBe("TOTODILE");
  });
});

describe("DataLoader menu asset loading", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("ensures species, move, and item maps are populated for menus", async () => {
    jest.resetModules();
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("pokemon_data.json")) {
        return JSON.stringify([SPECIES_FIXTURE]);
      }
      if (pathValue.endsWith("moves_data.json")) {
        return JSON.stringify({ TACKLE: MOVE_FIXTURE });
      }
      if (pathValue.endsWith("items.json")) {
        return JSON.stringify([ITEM_FIXTURE]);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader, speciesMap, movesMap, itemsMap } = await import("./data-loader");
    const loader = new DataLoader();
    loader.ensureMenuData();

    expect(speciesMap.get("TOTODILE")?.id).toBe("TOTODILE");
    expect(movesMap.get("TACKLE")?.name).toBe("TACKLE");
    expect(itemsMap.get("POTION")?.name).toBe("POTION");
  });

  it("throws when items.json is missing instead of proceeding with an empty item map", async () => {
    jest.resetModules();
    const warn = jest.spyOn(console, "warn").mockImplementation(() => {});
    jest.spyOn(fs.promises, "readFile").mockImplementation(async (pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("items.json")) {
        const error = new Error("missing items") as NodeJS.ErrnoException;
        error.code = "ENOENT";
        throw error;
      }
      return REAL_READ_FILE(pathLike, options as Parameters<typeof fs.promises.readFile>[1]);
    });

    const { loadItems } = await import("./data-loader");

    await expect(loadItems()).rejects.toThrow("ASM-backed item data is required;");
    expect(warn).not.toHaveBeenCalled();
  });

  it("throws when synchronous item loading cannot find items.json", async () => {
    jest.resetModules();
    const warn = jest.spyOn(console, "warn").mockImplementation(() => {});
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("items.json")) {
        const error = new Error("missing items") as NodeJS.ErrnoException;
        error.code = "ENOENT";
        throw error;
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { loadAllItems } = await import("./data-loader");

    expect(() => loadAllItems()).toThrow("ASM-backed item data is required;");
    expect(warn).not.toHaveBeenCalled();
  });
});

describe("DataLoader script loader parity", () => {
  afterEach(() => {
    jest.restoreAllMocks();
    jest.dontMock("./content-packs");
  });

  it("loads supplemental story events from the story_events directory", async () => {
    jest.resetModules();
    const storyEventsDir = `${path.sep}story_events`;
    const storyEventsPath = `${storyEventsDir}${path.sep}azalea_town.json`;
    const pcStringsPath = `assets${path.sep}data${path.sep}pc_strings.json`;

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return (
        pathValue.endsWith("story_events.json") ||
        pathValue.endsWith(storyEventsDir) ||
        pathValue.endsWith(storyEventsPath) ||
        pathValue.endsWith(pcStringsPath)
      );
    });
    jest
      .spyOn(fs, "readdirSync")
      .mockImplementation(((pathLike: fs.PathLike) => {
        const pathValue = String(pathLike);
        if (pathValue.endsWith(storyEventsDir)) {
          return [
            {
              name: "azalea_town.json",
              isFile: () => true,
            },
          ] as unknown as fs.Dirent[];
        }
        return [] as unknown as fs.Dirent[];
      }) as unknown as typeof fs.readdirSync);
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(storyEventsPath)) {
        return JSON.stringify({
          TestMap: {
            StoryEventScript: [{ command: "end", args: [] }],
          },
        });
      }
      if (pathValue.endsWith(pcStringsPath)) {
        return "{}";
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script("StoryEventScript")).toEqual([{ command: "end", args: [] }]);
  });

  it("loads flat single-map supplemental story event exports from the story_events directory", async () => {
    jest.resetModules();
    const storyEventsDir = `${path.sep}story_events`;
    const storyEventsPath = `${storyEventsDir}${path.sep}azalea_town.json`;
    const pcStringsPath = `assets${path.sep}data${path.sep}pc_strings.json`;

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return (
        pathValue.endsWith("story_events.json") ||
        pathValue.endsWith(storyEventsDir) ||
        pathValue.endsWith(storyEventsPath) ||
        pathValue.endsWith(pcStringsPath)
      );
    });
    jest
      .spyOn(fs, "readdirSync")
      .mockImplementation(((pathLike: fs.PathLike) => {
        const pathValue = String(pathLike);
        if (pathValue.endsWith(storyEventsDir)) {
          return [
            {
              name: "azalea_town.json",
              isFile: () => true,
            },
          ] as unknown as fs.Dirent[];
        }
        return [] as unknown as fs.Dirent[];
      }) as unknown as typeof fs.readdirSync);
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(storyEventsPath)) {
        return JSON.stringify({
          AzaleaTown_MapScripts: [{ command: "def_scene_scripts", args: [] }],
          AzaleaTownFlatScript: [{ command: "end", args: [] }],
        });
      }
      if (pathValue.endsWith(pcStringsPath)) {
        return "{}";
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.ensure_map_scripts("AzaleaTown")).not.toThrow();
    expect(loader.get_script("AzaleaTownFlatScript")).toEqual([{ command: "end", args: [] }]);
  });

  it("loads map script/event files from enabled content packs", async () => {
    jest.resetModules();
    jest.doMock("./content-packs", () => {
      const actual = jest.requireActual("./content-packs");
      return {
        ...actual,
        loadContentPackCategoryJsonSync: jest.fn((category: string) => {
          if (category === "maps") {
            return [
              {
                NewRoute_MapScripts: [{ command: "def_scene_scripts", args: [] }],
                NewRoute_MapEvents: [{ command: "def_warp_events", args: [] }],
                NewRouteScript: [{ command: "end", args: [] }],
              },
            ];
          }
          return [];
        }),
      };
    });
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return (
        pathValue.endsWith("story_events.json") ||
        pathValue.endsWith(`${path.sep}maps${path.sep}Route37.json`)
      );
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("story_events.json")) {
        return "{}";
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.ensure_map_scripts("NewRoute")).not.toThrow();
    expect(loader.get_script("NewRouteScript")).toEqual([{ command: "end", args: [] }]);
  });

  it("does not enumerate legacy per-map JSON files when a compiled content pack is enabled", async () => {
    jest.resetModules();
    const readTargets: string[] = [];

    jest.doMock("./content-packs", () => {
      const actual = jest.requireActual("./content-packs");
      return {
        ...actual,
        hasEnabledCompiledContentPackSync: jest.fn(() => true),
        loadContentPackCategoryJsonSync: jest.fn((category: string) => {
          if (category === "maps") {
            return [
              {
                Route37_MapScripts: [{ command: "def_scene_scripts", args: [] }],
                Route37_MapEvents: [{ command: "def_warp_events", args: [] }],
                Route37SignScript: [{ command: "end", args: [] }],
              },
            ];
          }
          return [];
        }),
      };
    });
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("story_events.json");
    });
    jest.spyOn(fs, "readdirSync").mockReturnValue([] as unknown as fs.Dirent[]);
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      readTargets.push(pathValue);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(`${path.sep}maps${path.sep}Route37.json`)) {
        throw new Error(`legacy map file should not be read: ${pathValue}`);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.ensure_map_scripts("Route37")).not.toThrow();
    expect(loader.get_script("Route37SignScript")).toEqual([{ command: "end", args: [] }]);
    expect(readTargets.some((target) => target.endsWith(`${path.sep}maps${path.sep}Route37.json`))).toBe(false);
  });

  it("throws when a supplemental story_events file is malformed instead of silently omitting its labels", async () => {
    jest.resetModules();
    const storyEventsDir = `${path.sep}story_events`;
    const storyEventsPath = `${storyEventsDir}${path.sep}azalea_town.json`;

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return (
        pathValue.endsWith("story_events.json") ||
        pathValue.endsWith(storyEventsDir) ||
        pathValue.endsWith(storyEventsPath)
      );
    });
    jest
      .spyOn(fs, "readdirSync")
      .mockImplementation(((pathLike: fs.PathLike) => {
        const pathValue = String(pathLike);
        if (pathValue.endsWith(storyEventsDir)) {
          return [
            {
              name: "azalea_town.json",
              isFile: () => true,
            },
          ] as unknown as fs.Dirent[];
        }
        return [] as unknown as fs.Dirent[];
      }) as unknown as typeof fs.readdirSync);
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(storyEventsPath)) {
        return "[]";
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();
    const loadScript = () => loader.get_script("StoryEventScript");

    expect(loadScript).toThrow(/ASM-backed story events are required;/);
    expect(loadScript).toThrow(/azalea_town\.json must contain an object\./);
  });

  it("throws when a supplemental story_events map payload is malformed instead of silently omitting that map", async () => {
    jest.resetModules();
    const storyEventsDir = `${path.sep}story_events`;
    const storyEventsPath = `${storyEventsDir}${path.sep}azalea_town.json`;
    const pcStringsPath = `assets${path.sep}data${path.sep}pc_strings.json`;

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return (
        pathValue.endsWith("story_events.json") ||
        pathValue.endsWith(storyEventsDir) ||
        pathValue.endsWith(storyEventsPath) ||
        pathValue.endsWith(pcStringsPath)
      );
    });
    jest
      .spyOn(fs, "readdirSync")
      .mockImplementation(((pathLike: fs.PathLike) => {
        const pathValue = String(pathLike);
        if (pathValue.endsWith(storyEventsDir)) {
          return [
            {
              name: "azalea_town.json",
              isFile: () => true,
            },
          ] as unknown as fs.Dirent[];
        }
        return [] as unknown as fs.Dirent[];
      }) as unknown as typeof fs.readdirSync);
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(storyEventsPath)) {
        return JSON.stringify({
          TestMap: "not_an_object",
        });
      }
      if (pathValue.endsWith(pcStringsPath)) {
        return "{}";
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.get_script("StoryEventScript")).toThrow(
      "ASM-backed story events are required; "
    );
    expect(() => loader.get_script("StoryEventScript")).toThrow(
      /azalea_town\.json entry TestMap must contain an object\./
    );
  });

  it("throws when a story event label payload is malformed instead of silently omitting it", async () => {
    jest.resetModules();

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return pathValue.endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return JSON.stringify({
          TestMap: {
            StoryEventScript: "not_an_array",
          },
        });
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.get_script("StoryEventScript")).toThrow(
      "ASM-backed story events are required; TestMap.StoryEventScript must be an array."
    );
  });

  it("loads pc_strings text entries into get_text", async () => {
    jest.resetModules();
    const pcStringsPath = `${path.sep}pc_strings.json`;

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return pathValue.endsWith("story_events.json") || pathValue.endsWith(pcStringsPath);
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(pcStringsPath)) {
        return JSON.stringify({ CustomPCText: "PC HELLO" });
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_text("CustomPCText")).toBe("PC HELLO");
  });

  it("throws when pc_strings.json is malformed instead of silently omitting PC text", async () => {
    jest.resetModules();
    const pcStringsPath = `${path.sep}pc_strings.json`;

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return pathValue.endsWith("story_events.json") || pathValue.endsWith(pcStringsPath);
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(pcStringsPath)) {
        return "[]";
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.get_text("CustomPCText")).toThrow(
      "ASM-backed PC string data is required; pc_strings.json must contain an object."
    );
  });

  it("throws when a pc_strings.json entry is malformed instead of coercing it into text", async () => {
    jest.resetModules();
    const pcStringsPath = `${path.sep}pc_strings.json`;

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return pathValue.endsWith("story_events.json") || pathValue.endsWith(pcStringsPath);
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return "{}";
      }
      if (pathValue.endsWith(pcStringsPath)) {
        return JSON.stringify({ CustomPCText: { text: "PC HELLO" } });
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.get_text("CustomPCText")).toThrow(
      "ASM-backed PC string data is required; pc_strings.json entry CustomPCText must be a string."
    );
  });

  it("indexes hidden item event flags for itemfinder checks", async () => {
    jest.resetModules();

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return pathValue.endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return JSON.stringify({
          TestMap: {
            HiddenItemScript: [
              { command: "hiddenitem", args: ["POTION", "EVENT_HIDDEN_ITEM"] },
            ],
          },
        });
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_hidden_item_event_flag("HiddenItemScript")).toBe("EVENT_HIDDEN_ITEM");
  });

  it("indexes trainer event flags for sightline scripts", async () => {
    jest.resetModules();

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return pathValue.endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return JSON.stringify({
          TestMap: {
            TrainerSightlineScript: [
              {
                command: "trainer",
                args: [
                  "BUG_CATCHER",
                  "BUG_CATCHER_TEST",
                  "EVENT_BEAT_BUG_CATCHER_TEST",
                  "SeenText",
                  "BeatenText",
                  "0",
                  ".Script",
                ],
              },
            ],
          },
        });
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();
    loader.get_script("TrainerSightlineScript");

    expect(loader.trainer_event_flags?.TrainerSightlineScript).toBe("EVENT_BEAT_BUG_CATCHER_TEST");
  });

  it("collects event flags across sjump and fallthrough successors", async () => {
    jest.resetModules();

    const storyPayload = {
      TestMap: {
        ScriptA: [{ command: "setevent", args: ["EVENT_A"] }],
        ScriptB: [
          { command: "conditional_event", args: ["EVENT_B"] },
          { command: "end", args: [] },
        ],
        ScriptJump: [{ command: "sjump", args: ["ScriptTarget"] }],
        ScriptTarget: [{ command: "checkevent", args: ["EVENT_C"] }],
      },
    };

    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      const pathValue = String(pathLike);
      return pathValue.endsWith("story_events.json");
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("story_events.json")) {
        return JSON.stringify(storyPayload);
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_script_event_flags("ScriptA").sort()).toEqual(["EVENT_A", "EVENT_B"]);
    expect(loader.get_script_event_flags("ScriptJump")).toEqual(["EVENT_C"]);
  });
});

describe("DataLoader mart loading", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("loads mart inventories from assets", async () => {
    jest.resetModules();
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      if (String(pathLike).endsWith("marts.json")) {
        return JSON.stringify({
          MART_VIOLET: [
            "POKE_BALL",
            "POTION",
            "ESCAPE_ROPE",
            "ANTIDOTE",
            "PARLYZ_HEAL",
            "AWAKENING",
            "X_DEFEND",
            "X_ATTACK",
            "X_SPEED",
            "FLOWER_MAIL",
          ],
        });
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();
    loader.loadMartData();

    expect(loader.martData?.get("MART_VIOLET")).toEqual([
      "POKE_BALL",
      "POTION",
      "ESCAPE_ROPE",
      "ANTIDOTE",
      "PARLYZ_HEAL",
      "AWAKENING",
      "X_DEFEND",
      "X_ATTACK",
      "X_SPEED",
      "FLOWER_MAIL",
    ]);
  });
});

describe("DataLoader NPC loading", () => {
  afterEach(() => {
    jest.dontMock("./content-packs");
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("loads merged NPC data into the map-indexed NPC table", async () => {
    jest.resetModules();
    jest.doMock("./content-packs", () => {
      const actual = jest.requireActual("./content-packs");
      return {
        ...actual,
        loadMergedNpcDataSync: jest.fn(() => ({
          TestMap: [{ script: "TestNpcScript", object_identifier: "TEST_MAP_NPC" }],
        })),
      };
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    loader.load_npc_data();

    expect(loader.npc_data.get("TestMap")).toEqual([
      { script: "TestNpcScript", object_identifier: "TEST_MAP_NPC" },
    ]);
  });

  it("throws when a merged NPC map entry is malformed instead of substituting an empty NPC list", async () => {
    jest.resetModules();
    jest.doMock("./content-packs", () => {
      const actual = jest.requireActual("./content-packs");
      return {
        ...actual,
        loadMergedNpcDataSync: jest.fn(() => ({ TestMap: { object_event: "NOT_AN_ARRAY" } })),
      };
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.load_npc_data()).toThrow(
      "ASM-backed NPC data is required; merged NPC data entry TestMap must be an array."
    );
  });
});

describe("DataLoader wild encounter loading", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("throws when merged wild encounter data is malformed instead of silently leaving encounters unloaded", async () => {
    jest.resetModules();
    jest.doMock("./content-packs", () => {
      const actual = jest.requireActual("./content-packs");
      return {
        ...actual,
        loadMergedWildEncountersSync: jest.fn(() => ({ ROUTE_29: { morning: [] } })),
      };
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.load_wild_encounter_data()).toThrow(
      "ASM-backed wild encounter data is required; merged encounter payload must be an array."
    );
  });
});

describe("DataLoader map metadata loading", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("throws when merged map attributes are malformed instead of silently leaving attributes unloaded", async () => {
    jest.resetModules();
    jest.doMock("./content-packs", () => {
      const actual = jest.requireActual("./content-packs");
      return {
        ...actual,
        loadMergedMapAttributesSync: jest.fn(() => null),
      };
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.load_map_attributes()).toThrow(
      "ASM-backed map attributes are required; merged map attributes must be an object."
    );
  });

  it("throws when merged map dimensions contain invalid width/height instead of silently skipping the map", async () => {
    jest.resetModules();
    jest.doMock("./content-packs", () => {
      const actual = jest.requireActual("./content-packs");
      return {
        ...actual,
        loadMergedMapDimensionsSync: jest.fn(() => ({
          ROUTE_29: { width: 0, height: 9 },
        })),
      };
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.load_map_dimensions()).toThrow(
      "ASM-backed map dimensions are required; ROUTE_29 must provide positive numeric width and height."
    );
  });
});

describe("DataLoader trainer rewards", () => {
  afterEach(() => {
    jest.dontMock("./content-packs");
    jest.restoreAllMocks();
  });

  it("resolves bundled trainer ids without trainer_constants.asm", async () => {
    jest.resetModules();
    jest.doMock("./content-packs", () => {
      const actual = jest.requireActual("./content-packs");
      return {
        ...actual,
        loadMergedTrainersSync: jest.fn(() => [
          {
            name: "DON@",
            trainer_id: "DON",
            trainer_class: "BUG_CATCHER",
            party: [],
            win_quote: "win",
            lose_quote: "lose",
            items: [],
            base_reward: 4,
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            encounter_music: "",
            ai_layers: [],
          },
        ]),
      };
    });
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("trainer_constants.asm")) {
        throw new Error("trainer_constants.asm should not be read for bundled trainer lookup");
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_trainer("DON")?.trainer_id).toBe("DON");
    expect(loader.get_trainer("DON@")?.trainer_id).toBe("DON");
  });

  it("uses exported trainer base rewards without reading ASM trainer attributes at runtime", async () => {
    jest.resetModules();
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (
        pathValue.endsWith("trainer_constants.asm") ||
        pathValue.endsWith("data/trainers/attributes.asm")
      ) {
        throw new Error("runtime must use exported trainer base rewards");
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });
    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(loader.get_trainer_base_reward("FALKNER1")).toBe(25);
    expect(loader.get_trainer_base_reward("JACK1")).toBe(8);
  });

  it("throws when trainers.json is malformed instead of silently keeping an empty trainer table", async () => {
    jest.resetModules();
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike, options) => {
      const pathValue = String(pathLike);
      if (pathValue.endsWith("trainers.json")) {
        return JSON.stringify({ FALKNER1: { name: "Falkner" } });
      }
      return REAL_READ_FILE_SYNC(pathLike, options as Parameters<typeof fs.readFileSync>[1]);
    });

    const { DataLoader } = await import("./data-loader");
    const loader = new DataLoader();

    expect(() => loader.load_trainer_data()).toThrow(
      /ASM-backed trainer data is required;/
    );
  });
});
