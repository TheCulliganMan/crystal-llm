import fs from "fs";
import path from "path";
import { exportCoreData } from "./index";

const decode2bppTileLevels = (data: Buffer, tileIndex: number): number[] => {
  const offset = tileIndex * 16;
  const levels: number[] = [];
  for (let row = 0; row < 8; row += 1) {
    const lo = data[offset + row * 2] ?? 0;
    const hi = data[offset + row * 2 + 1] ?? 0;
    for (let col = 0; col < 8; col += 1) {
      const mask = 1 << (7 - col);
      levels.push(((hi & mask) ? 2 : 0) | ((lo & mask) ? 1 : 0));
    }
  }
  return levels;
};

describe("core exporters integration", () => {
  it("writes canonical JSON outputs for the TypeScript asset tree", () => {
    exportCoreData();

    const dataDir = path.resolve(__dirname, "..", "..", "..", "..", "apps", "web", "assets", "data");
    const assetRoot = path.dirname(dataDir);

    const requiredJsonPaths = [
      "pokemon_data.json",
      "moves_data.json",
      "learnsets.json",
      "level_up_moves.json",
      "egg_moves.json",
      "items.json",
      "evolutions.json",
      "animations.json",
      "flee_mons.json",
      "wild_encounters.json",
      "trainers.json",
      "map_attributes.json",
      "map_dimensions.json",
      "marts.json",
      "menu_icons.json",
      "npcs.json",
      "pokedex.json",
      "pokedex_entries.json",
      "pokegear_landmarks.json",
      "pokegear_town_map_palette_map.json",
      "pc_strings.json",
      "content-packs/index.json",
    ];
    for (const relativePath of requiredJsonPaths) {
      const targetPath = path.join(dataDir, relativePath);
      expect(fs.existsSync(targetPath)).toBe(true);
      expect(fs.readFileSync(targetPath, "utf8").trim()).not.toBe("");
    }

    const pokemon = JSON.parse(fs.readFileSync(path.join(dataDir, "pokemon_data.json"), "utf8")) as Array<{
      id?: string;
      int_id?: number;
      base_stats?: Record<string, number>;
    }>;
    expect(pokemon.length).toBeGreaterThanOrEqual(251);
    expect(pokemon).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: "TOTODILE",
          int_id: 158,
          base_stats: expect.objectContaining({ hp: expect.any(Number) }),
        }),
      ])
    );

    const moves = JSON.parse(fs.readFileSync(path.join(dataDir, "moves_data.json"), "utf8")) as Record<
      string,
      { name?: string; type?: string; pp?: number }
    >;
    expect(moves.TACKLE).toEqual(
      expect.objectContaining({ name: "TACKLE", type: "NORMAL", pp: expect.any(Number) })
    );

    const learnsets = JSON.parse(fs.readFileSync(path.join(dataDir, "learnsets.json"), "utf8")) as Record<
      string,
      Array<[number, string]>
    >;
    expect(learnsets.CYNDAQUIL).toEqual(
      expect.arrayContaining([
        [1, "TACKLE"],
        [1, "LEER"],
      ])
    );

    const levelUpMoves = JSON.parse(fs.readFileSync(path.join(dataDir, "level_up_moves.json"), "utf8")) as Record<
      string,
      Array<{ level: number; move: string }>
    >;
    expect(levelUpMoves.CYNDAQUIL).toEqual(
      expect.arrayContaining([
        { level: 1, move: "TACKLE" },
        { level: 1, move: "LEER" },
      ])
    );

    const eggMoves = JSON.parse(fs.readFileSync(path.join(dataDir, "egg_moves.json"), "utf8")) as Record<
      string,
      string[]
    >;
    expect(eggMoves.TOTODILE).toEqual(expect.arrayContaining(["CRUNCH"]));

    const items = JSON.parse(fs.readFileSync(path.join(dataDir, "items.json"), "utf8")) as Array<{
      name?: string;
      script_name?: string;
      effect?: string;
      pocket?: string;
      description?: string;
      tmhm_index: number | null;
    }>;
    expect(items).toHaveLength(255);
    expect(items).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          name: "POKEGEAR",
          effect: "NONE",
          pocket: "KEY_ITEM",
          description: expect.stringContaining("map"),
          tmhm_index: null,
        }),
      ])
    );
    expect(items).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          name: "TM41",
          script_name: "TM_THUNDERPUNCH",
          effect: "NONE",
          pocket: "TM_HM",
          tmhm_index: 41,
        }),
        expect.objectContaining({
          name: "HM03",
          script_name: "HM_SURF",
          effect: "NONE",
          pocket: "TM_HM",
          tmhm_index: 53,
        }),
      ])
    );

    const fleeMons = JSON.parse(fs.readFileSync(path.join(dataDir, "flee_mons.json"), "utf8")) as {
      buckets: Record<string, string[]>;
    };
    expect(fleeMons.buckets.always).toEqual(expect.arrayContaining(["RAIKOU", "ENTEI"]));
    expect(fleeMons.buckets.often).toContain("DELIBIRD");
    expect(fleeMons.buckets.sometimes).toContain("MAGNEMITE");

    const marts = JSON.parse(fs.readFileSync(path.join(dataDir, "marts.json"), "utf8")) as Record<string, string[]>;
    expect(marts.MART_CHERRYGROVE).toEqual(["POTION", "ANTIDOTE", "PARLYZ_HEAL", "AWAKENING"]);
    expect(marts.MART_GOLDENROD_5F_4).toEqual(
      expect.arrayContaining(["TM_THUNDERPUNCH", "TM_HEADBUTT", "TM_ROCK_SMASH"])
    );

    const menuIcons = JSON.parse(fs.readFileSync(path.join(dataDir, "menu_icons.json"), "utf8")) as Record<string, string>;
    expect(Object.keys(menuIcons).length).toBeGreaterThanOrEqual(252);
    expect(menuIcons.BULBASAUR).toBe("ICON_BULBASAUR");
    expect(menuIcons.HO_OH).toBe("ICON_HO_OH");
    expect(menuIcons.EGG).toBe("ICON_EGG");

    const pcStrings = JSON.parse(fs.readFileSync(path.join(dataDir, "pc_strings.json"), "utf8")) as Record<string, string>;
    expect(pcStrings.PCString_ChooseaPKMN).toBe("Choose a <PK><MN>.");
    expect(pcStrings.PCString_NoReleasingEGGS).toBe("No releasing EGGS!");

    const pokedexEntries = JSON.parse(
      fs.readFileSync(path.join(dataDir, "pokedex_entries.json"), "utf8")
    ) as Record<string, { species: string; classification: string; heightDigits: number; weightDigits: number; pages: string[] }>;
    expect(pokedexEntries.CHIKORITA).toEqual(
      expect.objectContaining({
        species: "CHIKORITA",
        classification: "LEAF",
        heightDigits: 211,
        weightDigits: 140,
        pages: expect.arrayContaining([expect.stringContaining("sunlight")]),
      })
    );

    const evolutions = JSON.parse(fs.readFileSync(path.join(dataDir, "evolutions.json"), "utf8")) as Array<{
      species?: string;
    }>;
    expect(evolutions).toEqual(
      expect.arrayContaining([expect.objectContaining({ species: "TOTODILE" })])
    );

    const wildEncounters = JSON.parse(
      fs.readFileSync(path.join(dataDir, "wild_encounters.json"), "utf8")
    ) as Array<{ map_name?: string; grass_rates?: unknown; grass?: unknown; water?: unknown }>;
    expect(wildEncounters).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          map_name: "Route29",
          grass_rates: expect.any(Object),
          grass: expect.any(Object),
          water: expect.any(Object),
        }),
      ])
    );

    const trainers = JSON.parse(fs.readFileSync(path.join(dataDir, "trainers.json"), "utf8")) as Array<{
      trainer_id?: string;
      base_reward?: number;
      party?: unknown[];
    }>;
    expect(trainers).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          trainer_id: expect.any(String),
          base_reward: expect.any(Number),
          party: expect.any(Array),
        }),
      ])
    );
    expect(trainers.find((trainer) => trainer.trainer_id === "JACK1")?.base_reward).toBe(8);
    expect(trainers.every((trainer) => typeof trainer.base_reward === "number" && trainer.base_reward > 0)).toBe(true);

    const mapAttributes = JSON.parse(
      fs.readFileSync(path.join(dataDir, "map_attributes.json"), "utf8")
    ) as Record<string, { connections?: unknown[]; map_constant?: string; map_events_label?: string }>;
    expect(mapAttributes.Route29).toEqual(
      expect.objectContaining({
        map_constant: "ROUTE_29",
        map_events_label: "Route29_MapEvents",
        connections: expect.arrayContaining([
          expect.objectContaining({ target_map: "CherrygroveCity" }),
          expect.objectContaining({ target_map: "NewBarkTown" }),
        ]),
      })
    );

    const mapDimensions = JSON.parse(
      fs.readFileSync(path.join(dataDir, "map_dimensions.json"), "utf8")
    ) as Record<string, { width?: number; height?: number }>;
    expect(mapDimensions.ROUTE_29).toEqual(
      expect.objectContaining({ width: expect.any(Number), height: expect.any(Number) })
    );

    const npcData = JSON.parse(fs.readFileSync(path.join(dataDir, "npcs.json"), "utf8")) as Record<
      string,
      Array<{ script?: string; object_identifier?: string }>
    >;
    expect(npcData.Route29).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          script: "Route29YoungsterScript",
          object_identifier: expect.any(String),
        }),
      ])
    );

    const pokedex = JSON.parse(fs.readFileSync(path.join(dataDir, "pokedex.json"), "utf8")) as Array<{
      species?: string;
      text?: string;
    }>;
    expect(pokedex).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ species: "TOTODILE", text: expect.any(String) }),
      ])
    );

    const pokegearLandmarks = JSON.parse(
      fs.readFileSync(path.join(dataDir, "pokegear_landmarks.json"), "utf8")
    ) as {
      landmarks: Array<{ id: number; constant: string; name: string; region: string }>;
      map_to_landmark: Record<string, string>;
    };

    expect(pokegearLandmarks.landmarks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: 1,
          constant: "LANDMARK_NEW_BARK_TOWN",
          name: "NEW BARK TOWN",
          region: "JOHTO",
        }),
        expect.objectContaining({
          constant: "LANDMARK_PALLET_TOWN",
          name: "PALLET TOWN",
          region: "KANTO",
        }),
      ])
    );
    expect(pokegearLandmarks.map_to_landmark.PlayersHouse2F).toBe("LANDMARK_NEW_BARK_TOWN");
    expect(pokegearLandmarks.map_to_landmark.PalletTown).toBe("LANDMARK_PALLET_TOWN");

    const pokegearPaletteMap = JSON.parse(
      fs.readFileSync(path.join(dataDir, "pokegear_town_map_palette_map.json"), "utf8")
    ) as Record<string, string[]>;
    expect(pokegearPaletteMap.town_map).toHaveLength(48);
    expect(pokegearPaletteMap.pokegear).toHaveLength(48);
    expect(pokegearPaletteMap.town_map).toContain("POI_MTN");

    const animations = JSON.parse(
      fs.readFileSync(path.join(dataDir, "animations.json"), "utf8")
    ) as Record<string, string[]>;
    expect(animations.BattleAnim_Tackle).toEqual(
      expect.arrayContaining([
        expect.stringMatching(/^anim_1gfx\b/),
        expect.stringMatching(/^anim_obj\b/),
      ])
    );

    const contentPackIndex = JSON.parse(
      fs.readFileSync(path.join(dataDir, "content-packs", "index.json"), "utf8")
    ) as {
      packs: Array<{
        id: string;
        enabled?: boolean;
        priority?: number;
        files: Record<string, string[]>;
      }>;
    };
    const corePack = contentPackIndex.packs.find((pack) => pack.id === "core-modular");
    const route29Pack = contentPackIndex.packs.find((pack) => pack.id === "module-route-Route29");
    expect(corePack).toEqual(
      expect.objectContaining({
        enabled: true,
        priority: -100,
        files: expect.objectContaining({
          map_scripts: expect.arrayContaining([
            "content-packs/core-modular/map_scripts/Route29.json",
          ]),
          map_blocks: expect.arrayContaining([
            "content-packs/core-modular/map_blocks/Route29_Blocks.json",
          ]),
          map_attributes: expect.arrayContaining([
            "content-packs/core-modular/map_attributes/Route29.json",
          ]),
          map_dimensions: expect.arrayContaining([
            "content-packs/core-modular/map_dimensions/ROUTE_29.json",
          ]),
          wild_encounters: expect.arrayContaining([
            "content-packs/core-modular/wild_encounters/Route29.json",
          ]),
          npcs: expect.arrayContaining(["content-packs/core-modular/npcs/Route29.json"]),
        }),
      })
    );
    expect(route29Pack).toEqual(
      expect.objectContaining({
        enabled: false,
        files: expect.objectContaining({
          map_scripts: ["content-packs/core-modular/map_scripts/Route29.json"],
          map_blocks: ["content-packs/core-modular/map_blocks/Route29_Blocks.json"],
          map_attributes: ["content-packs/core-modular/map_attributes/Route29.json"],
        }),
      })
    );
    expect(contentPackIndex.packs.filter((pack) => pack.id.startsWith("module-")).every(
      (pack) => pack.enabled === false
    )).toBe(true);

    const route29Map = JSON.parse(
      fs.readFileSync(path.join(dataDir, "content-packs", "core-modular", "map_scripts", "Route29.json"), "utf8")
    ) as Record<string, unknown>;
    expect(Array.isArray(route29Map)).toBe(false);
    expect(route29Map.Route29_MapScripts).toEqual(expect.any(Array));
    expect(route29Map.Route29_MapEvents).toEqual(expect.any(Array));
    expect(route29Map.Route29YoungsterText).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ command: "text", args: expect.stringContaining("Yo.") }),
        expect.objectContaining({ command: "line", args: expect.stringContaining("#MON") }),
      ])
    );

    const requiredCoreCategories = [
      "pokemon",
      "moves",
      "learnsets",
      "level_up_moves",
      "egg_moves",
      "evolutions",
      "map_scripts",
      "map_blocks",
      "map_attributes",
      "map_dimensions",
      "wild_encounters",
      "npcs",
      "pokegear_landmarks",
      "items",
      "trainers",
      "pokedex",
      "phone_scripts",
      "audio",
    ];
    for (const category of requiredCoreCategories) {
      expect(corePack?.files[category]?.length ?? 0).toBeGreaterThan(0);
    }

    for (const [category, files] of Object.entries(corePack?.files ?? {})) {
      for (const relativePath of files) {
        const targetPath = relativePath.startsWith("content-packs/")
          ? path.join(dataDir, relativePath)
          : path.join(dataDir, relativePath);
        expect(fs.existsSync(targetPath)).toBe(true);
        if (category === "audio") {
          expect(relativePath.endsWith(".json")).toBe(true);
          const metadata = JSON.parse(fs.readFileSync(targetPath, "utf8")) as Record<
            string,
            { path: string }
          >;
          const [entry] = Object.values(metadata);
          expect(entry.path.endsWith(".mid")).toBe(true);
          const bytes = fs.readFileSync(path.join(dataDir, entry.path));
          expect(bytes.subarray(0, 4).toString("ascii")).toBe("MThd");
          continue;
        }
        if (category !== "map_blocks") {
          expect(fs.readFileSync(targetPath, "utf8").trim()).not.toBe("");
        }
        const payload = JSON.parse(fs.readFileSync(targetPath, "utf8"));
        const firstPayloadEntry = () => Object.values(payload)[0] as Record<string, unknown>;
        if (category === "pokemon") {
          expect(firstPayloadEntry()).toEqual(expect.objectContaining({ id: expect.any(String) }));
        } else if (category === "moves") {
          expect(firstPayloadEntry()).toEqual(expect.objectContaining({ name: expect.any(String) }));
        } else if (category === "learnsets") {
          expect(firstPayloadEntry()).toEqual(expect.objectContaining({ species: expect.any(String), learnset: expect.any(Array) }));
        } else if (category === "level_up_moves" || category === "egg_moves") {
          expect(firstPayloadEntry()).toEqual(expect.objectContaining({ species: expect.any(String), moves: expect.any(Array) }));
        } else if (category === "evolutions") {
          expect(firstPayloadEntry()).toEqual(expect.objectContaining({ species: expect.any(String) }));
        } else if (category === "map_scripts") {
          expect(payload && typeof payload === "object" && !Array.isArray(payload)).toBe(true);
          expect(Object.keys(payload).some((key) => key.endsWith("_MapScripts"))).toBe(true);
          expect(Object.keys(payload).some((key) => key.endsWith("_MapEvents"))).toBe(true);
        } else if (category === "map_blocks") {
          expect(payload && typeof payload === "object" && !Array.isArray(payload)).toBe(true);
          expect(Object.values(payload).every((value) => typeof value === "string")).toBe(true);
        } else if (category === "map_attributes") {
          const [entry] = Object.values(payload) as Array<Record<string, unknown>>;
          expect(entry).toEqual(
            expect.objectContaining({
              map_constant: expect.any(String),
              connections: expect.any(Array),
            })
          );
        } else if (category === "map_dimensions") {
          const [entry] = Object.values(payload) as Array<Record<string, unknown>>;
          expect(entry).toEqual(
            expect.objectContaining({ width: expect.any(Number), height: expect.any(Number) })
          );
        } else if (category === "wild_encounters") {
          expect(firstPayloadEntry()).toEqual(expect.objectContaining({ map_name: expect.any(String) }));
        } else if (category === "npcs") {
          expect(payload && typeof payload === "object" && !Array.isArray(payload)).toBe(true);
          expect(Object.values(payload).every((value) => Array.isArray(value))).toBe(true);
        } else if (category === "pokegear_landmarks") {
          expect(payload).toEqual(
            expect.objectContaining({
              landmarks: expect.any(Array),
              map_to_landmark: expect.any(Object),
            })
          );
        } else if (category === "items") {
          expect(firstPayloadEntry()).toEqual(expect.objectContaining({ name: expect.any(String) }));
        } else if (category === "trainers") {
          expect(firstPayloadEntry()).toEqual(
            expect.objectContaining({ trainer_id: expect.any(String), party: expect.any(Array) })
          );
        } else if (category === "pokedex") {
          expect(firstPayloadEntry()).toEqual(expect.objectContaining({ species: expect.any(String) }));
        } else if (category === "phone_scripts") {
          expect(payload && typeof payload === "object" && !Array.isArray(payload)).toBe(true);
          expect(Object.values(payload).every((value) => Array.isArray(value))).toBe(true);
        }
      }
    }

    const graphicsSentinel = path.resolve(
      assetRoot,
      "gfx",
      "tilesets",
      "bg_tiles.pal"
    );
    expect(fs.existsSync(graphicsSentinel)).toBe(true);
    expect(fs.readFileSync(graphicsSentinel, "utf8").trim()).not.toBe("");
    expect(
      fs.existsSync(
        path.resolve(
          assetRoot,
          "gfx",
          "tilesets",
          "whirlpool",
          "1.2bpp"
        )
      )
    ).toBe(true);
    expect(
      fs.existsSync(
        path.resolve(
          assetRoot,
          "gfx",
          "font",
          "font_battle_extra.2bpp"
        )
      )
    ).toBe(true);
    const fontExtra = fs.readFileSync(path.resolve(assetRoot, "gfx", "font", "font_extra.2bpp"));
    const ellipsisTile = decode2bppTileLevels(fontExtra, 21);
    expect(ellipsisTile[0]).toBe(0);
    expect(ellipsisTile.filter((level) => level === 0).length).toBeGreaterThan(50);
    expect(ellipsisTile.some((level) => level > 0)).toBe(true);
  });
});
