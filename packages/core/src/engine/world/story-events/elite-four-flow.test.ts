import { DataLoader, type ScriptData } from "@pokecrystal/core/core/data-loader";

const commands = (script: ScriptData | null): string[] => (script ?? []).map((entry) => String(entry.command ?? ""));

const argsFor = (script: ScriptData, command: string): unknown[][] =>
  script
    .filter((entry) => entry.command === command)
    .map((entry) => (Array.isArray(entry.args) ? entry.args : []));

describe("Elite Four story flow", () => {
  it("loads the Crystal Elite Four and Champion parties in ASM order", () => {
    const loader = new DataLoader();
    const parties = {
      WILL1: [
        ["XATU", 40],
        ["JYNX", 41],
        ["EXEGGUTOR", 41],
        ["SLOWBRO", 41],
        ["XATU", 42],
      ],
      KOGA1: [
        ["ARIADOS", 40],
        ["VENOMOTH", 41],
        ["FORRETRESS", 43],
        ["MUK", 42],
        ["CROBAT", 44],
      ],
      BRUNO1: [
        ["HITMONTOP", 42],
        ["HITMONLEE", 42],
        ["HITMONCHAN", 42],
        ["ONIX", 43],
        ["MACHAMP", 46],
      ],
      KAREN1: [
        ["UMBREON", 42],
        ["VILEPLUME", 42],
        ["GENGAR", 45],
        ["MURKROW", 44],
        ["HOUNDOOM", 47],
      ],
      LANCE: [
        ["GYARADOS", 44],
        ["DRAGONITE", 47],
        ["DRAGONITE", 47],
        ["AERODACTYL", 46],
        ["CHARIZARD", 46],
        ["DRAGONITE", 50],
      ],
    } as const;

    for (const [trainerId, expectedParty] of Object.entries(parties)) {
      const trainer = loader.get_trainer(trainerId);
      expect(trainer).toBeDefined();
      expect(trainer!.party.map((mon) => [mon.species.id, mon.level])).toEqual(expectedParty);
      expect(trainer!.party.every((mon) => mon.moves.length === 4)).toBe(true);
    }
  });

  it("preserves Lance's right-side approach into the battle script", () => {
    const loader = new DataLoader();
    const script = loader.get_script("Script_ApproachLanceFromRight");
    expect(script).not.toBeNull();
    expect(commands(script)).toEqual(["special", "applymovement", "sjump"]);
    expect(script).toContainEqual({ command: "sjump", args: ["LancesRoomLanceScript"] });
  });

  it("keeps every Elite Four battle script wired to its trainer, event flag, and exit door", () => {
    const loader = new DataLoader();
    const rooms = [
      {
        mapEvents: "WillsRoom_MapEvents",
        script: "WillScript_Battle",
        trainer: ["WILL", "WILL1"],
        beatFlag: "EVENT_BEAT_ELITE_4_WILL",
        exitFlag: "EVENT_WILLS_ROOM_EXIT_OPEN",
        nextWarp: "KOGAS_ROOM",
      },
      {
        mapEvents: "KogasRoom_MapEvents",
        script: "KogaScript_Battle",
        trainer: ["KOGA", "KOGA1"],
        beatFlag: "EVENT_BEAT_ELITE_4_KOGA",
        exitFlag: "EVENT_KOGAS_ROOM_EXIT_OPEN",
        nextWarp: "BRUNOS_ROOM",
      },
      {
        mapEvents: "BrunosRoom_MapEvents",
        script: "BrunoScript_Battle",
        trainer: ["BRUNO", "BRUNO1"],
        beatFlag: "EVENT_BEAT_ELITE_4_BRUNO",
        exitFlag: "EVENT_BRUNOS_ROOM_EXIT_OPEN",
        nextWarp: "KARENS_ROOM",
      },
      {
        mapEvents: "KarensRoom_MapEvents",
        script: "KarenScript_Battle",
        trainer: ["KAREN", "KAREN1"],
        beatFlag: "EVENT_BEAT_ELITE_4_KAREN",
        exitFlag: "EVENT_KARENS_ROOM_EXIT_OPEN",
        nextWarp: "LANCES_ROOM",
      },
    ];

    for (const room of rooms) {
      const script = loader.get_script(room.script);
      expect(script).not.toBeNull();
      expect(argsFor(script!, "loadtrainer")).toContainEqual(room.trainer);
      expect(argsFor(script!, "setevent")).toContainEqual([room.beatFlag]);
      expect(argsFor(script!, "setevent")).toContainEqual([room.exitFlag]);
      expect(commands(script!)).toEqual(
        expect.arrayContaining(["winlosstext", "startbattle", "reloadmapafterbattle", "changeblock"]),
      );

      const mapEvents = loader.get_script(room.mapEvents);
      expect(JSON.stringify(mapEvents)).toContain(room.nextWarp);
    }
  });

  it("sets the post-League world flags before registering Hall of Fame", () => {
    const loader = new DataLoader();
    const script = loader.get_script("HallOfFameEnterScript");
    expect(script).not.toBeNull();
    const setEvents = argsFor(script!, "setevent");
    const clearEvents = argsFor(script!, "clearevent");

    expect(setEvents).toContainEqual(["EVENT_BEAT_ELITE_FOUR"]);
    expect(setEvents).toContainEqual(["EVENT_TELEPORT_GUY"]);
    expect(setEvents).toContainEqual(["EVENT_RIVAL_SPROUT_TOWER"]);
    expect(clearEvents).toContainEqual(["EVENT_RED_IN_MT_SILVER"]);
    expect(argsFor(script!, "special")).toContainEqual(["HealParty"]);
    expect(commands(script!).indexOf("halloffame")).toBeGreaterThan(commands(script!).indexOf("special"));
  });
});
