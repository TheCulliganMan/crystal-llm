import fs from 'fs';
import { promises as fsPromises } from 'fs';
import * as path from 'path';
import { z } from 'zod';
import {
  getTilesetCollisionPath,
  getTilesetMetatilesPath,
  getTilesetPaletteMapJsonPath,
} from './paths';
import { resolveCollisionValue } from '@pokecrystal/core/engine/world/overworld/collision-data';

export const METATILE_WIDTH = 4;
const DEFAULT_COLLISION_VALUE = resolveCollisionValue('FLOOR');

const RawCollisionDataSchema = z.record(z.string(), z.array(z.union([z.number(), z.string()])));
const RawPaletteMapSchema = z.union([
  z.array(z.number()),
  z.record(
    z.string(),
    z.union([z.number(), z.string(), z.array(z.union([z.number(), z.string()]))])
  ),
]);

interface Metatile {
  collision: readonly number[];
}

function parseAsmPaletteMap(content: string): Map<number, number> {
  const parsedMap = new Map<number, number>();
  const lines = content.split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith(';') || !trimmed) {
      continue;
    }
    const parts = trimmed.split(/\s+/);
    if (parts.length >= 2 && parts[0].toLowerCase() === 'metatile') {
      try {
        const metatileId = parseInt(parts[1].replace(',', ''), 16);
        const paletteValue = parseInt(parts[2], 0);
        if (!isNaN(metatileId) && !isNaN(paletteValue)) {
          parsedMap.set(metatileId, paletteValue);
        }
      } catch (_error) {
        // Malformed lines are common in the source .asm files.
        // It's safer to ignore them than to crash the data loader.
      }
    }
  }
  return parsedMap;
}

export class Tileset {
  public readonly tilesetName: string;
  public readonly metatiles: readonly Metatile[];
  public readonly paletteMap: ReadonlyMap<number, number>;

  private constructor(
    tilesetName: string,
    metatiles: readonly Metatile[],
    paletteMap: ReadonlyMap<number, number>
  ) {
    this.tilesetName = tilesetName;
    this.metatiles = metatiles;
    this.paletteMap = paletteMap;
  }

  public static async fromTilesetName(tilesetName: string): Promise<Tileset> {
    const collisionMap = await this.loadCollisionData(tilesetName);
    const paletteMap = await this.loadPaletteMap(tilesetName);
    const metatileCount = await this.loadMetatileCount(tilesetName);

    const metatiles: Metatile[] = [];
    for (let i = 0; i < metatileCount; i++) {
      const collision =
        collisionMap.get(i) ??
        new Array(4).fill(DEFAULT_COLLISION_VALUE);
      while (collision.length < 4) {
        collision.push(DEFAULT_COLLISION_VALUE);
      }
      metatiles.push({ collision });
    }

    return new Tileset(tilesetName, metatiles, paletteMap);
  }

  public static createPlaceholder(tilesetName: string): Tileset {
    return new Tileset(tilesetName, [], new Map());
  }

  private static async loadCollisionData(
    tilesetName: string
  ): Promise<Map<number, number[]>> {
    const collisionPath = getTilesetCollisionPath(tilesetName);
    try {
      const content = await fsPromises.readFile(collisionPath, 'utf8');
      const raw = JSON.parse(content);
      const validated = RawCollisionDataSchema.parse(raw);
      const parsed = new Map<number, number[]>();
      for (const [key, value] of Object.entries(validated)) {
        const metatileId = parseInt(key, 16);
        if (!isNaN(metatileId)) {
          parsed.set(
            metatileId,
            value.map((entry) =>
              typeof entry === "string" ? resolveCollisionValue(entry) : entry
            )
          );
        }
      }
      return parsed;
    } catch (error: unknown) {
      const err = error as NodeJS.ErrnoException | undefined;
      if (err?.code === 'ENOENT') {
        return new Map();
      }
      const message = err?.message ?? String(error);
      throw new Error(
        `Failed to load collision data for ${tilesetName}: ${message}`
      );
    }
  }

  private static async loadPaletteMap(
    tilesetName: string
  ): Promise<Map<number, number>> {
    const jsonPath = getTilesetPaletteMapJsonPath(tilesetName);

    try {
      const content = await fsPromises.readFile(jsonPath, 'utf8');
      const rawMap = JSON.parse(content);
      const parsedMap = new Map<number, number>();
      const validated = RawPaletteMapSchema.parse(rawMap);
      if (Array.isArray(validated)) {
        validated.forEach((paletteValue, tileId) => {
          parsedMap.set(tileId, paletteValue);
        });
        return parsedMap;
      }
      for (const [key, value] of Object.entries(validated)) {
        const metatileId = parseInt(key, 16);
        if (isNaN(metatileId)) continue;

        let paletteValueSource: string | number | undefined;
        if (Array.isArray(value)) {
          if (value.length > 0) {
            paletteValueSource = value[0];
          }
        } else {
          paletteValueSource = value;
        }

        if (paletteValueSource !== undefined) {
          const paletteValue =
            typeof paletteValueSource === 'string'
              ? parseInt(paletteValueSource, 0)
              : paletteValueSource;
          if (!isNaN(paletteValue)) {
            parsedMap.set(metatileId, paletteValue);
          }
        }
      }
      return parsedMap;
    } catch (error: unknown) {
      const err = error as NodeJS.ErrnoException | undefined;
      if (err?.code !== 'ENOENT') {
        const message = err?.message ?? String(error);
        throw new Error(
          `Failed to parse palette map JSON for ${tilesetName}: ${message}`
        );
      }

      return new Map();
    }
  }

  private static async loadMetatileCount(
    tilesetName: string
  ): Promise<number> {
    const metatilesPath = getTilesetMetatilesPath(tilesetName);
    try {
      const data = await fsPromises.readFile(metatilesPath);
      if (data.length % 16 !== 0) {
        throw new Error(
          `Metatile data for ${tilesetName} has unexpected length ${data.length}`
        );
      }
      return data.length / 16;
    } catch (error: unknown) {
      throw error;
    }
  }
}
