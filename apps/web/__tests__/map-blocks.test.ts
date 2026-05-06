import fs from "fs";
import path from "path";

import { resolveMapBlockPath } from "@/core/map-blocks";
import { getDataDir } from "@/core/paths";

const normalize = (value: string): string => value.replace(/\\/g, "/");

describe("map block resolution", () => {
  it("resolves GuideGentsHouse blocks to House1.blk", () => {
    const resolved = resolveMapBlockPath("GuideGentsHouse", "GuideGentsHouse_Blocks");
    expect(normalize(resolved).endsWith("/maps/House1.blk")).toBe(true);
  });

  it("resolves block files for every map attribute entry", () => {
    const mapAttributesPath = path.join(getDataDir(), "map_attributes.json");
    const mapAttributes = JSON.parse(fs.readFileSync(mapAttributesPath, "utf8")) as Record<
      string,
      { blocks_label?: string | null }
    >;
    const missing: string[] = [];

    for (const [mapName, attributes] of Object.entries(mapAttributes)) {
      try {
        const blockPath = resolveMapBlockPath(mapName, attributes?.blocks_label ?? null);
        if (!fs.existsSync(blockPath)) {
          missing.push(`${mapName} -> ${blockPath}`);
        }
      } catch (error) {
        missing.push(`${mapName} -> ${error instanceof Error ? error.message : String(error)}`);
      }
    }

    expect(missing).toEqual([]);
  });
});
