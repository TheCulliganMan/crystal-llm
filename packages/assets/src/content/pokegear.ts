
import { z } from 'zod';
import fs from 'fs';
import { promises as fsPromises } from 'fs';
import { getDataDir } from '@pokecrystal/core/core/paths';
import path from 'path';
import { mergePokegearLandmarksPayload } from '@pokecrystal/core/core/content-packs';

export const LandmarkEntrySchema = z.object({
  id: z.number(),
  constant: z.string(),
  label: z.string(),
  name: z.string(),
  x: z.number(),
  y: z.number(),
  region: z.string(),
});

export const PokegearLandmarkPayloadSchema = z.object({
  landmarks: z.array(LandmarkEntrySchema),
  map_to_landmark: z.record(z.string(), z.string()),
});

export type LandmarkEntry = z.infer<typeof LandmarkEntrySchema>;

let cachedPayload: z.infer<typeof PokegearLandmarkPayloadSchema> | null = null;
let cachedPayloadPromise: Promise<z.infer<typeof PokegearLandmarkPayloadSchema>> | null = null;

async function loadPayload() {
  if (cachedPayload) {
    return cachedPayload;
  }
  if (cachedPayloadPromise) {
    return cachedPayloadPromise;
  }

  cachedPayloadPromise = (async () => {
    const dataDir = await getDataDir();
    const filePath = path.join(dataDir, 'pokegear_landmarks.json');

    try {
      const rawText = await fsPromises.readFile(filePath, 'utf-8');
      const merged = mergePokegearLandmarksPayload(JSON.parse(rawText));
      const payload = PokegearLandmarkPayloadSchema.parse(merged);
      cachedPayload = payload;
      return payload;
    } catch (error) {
      if (error instanceof Error) {
        throw new Error(`Invalid Pokégear landmark JSON content in ${filePath}: ${error.message}`);
      }
      throw new Error(`An unknown error occurred while loading ${filePath}`);
    } finally {
      cachedPayloadPromise = null;
    }
  })();

  return cachedPayloadPromise;
}

export function loadPokegearPayloadSync() {
  if (cachedPayload) {
    return cachedPayload;
  }
  const dataDir = getDataDir();
  const filePath = path.join(dataDir, 'pokegear_landmarks.json');
  try {
    const rawText = fs.readFileSync(filePath, 'utf-8');
    const merged = mergePokegearLandmarksPayload(JSON.parse(rawText));
    const payload = PokegearLandmarkPayloadSchema.parse(merged);
    cachedPayload = payload;
    return payload;
  } catch (error) {
    if (error instanceof Error) {
      throw new Error(`Invalid Pok\u00e9gear landmark JSON content in ${filePath}: ${error.message}`);
    }
    throw new Error(`An unknown error occurred while loading ${filePath}`);
  }
}

export async function getPokegearLandmarks(): Promise<LandmarkEntry[]> {
  const payload = await loadPayload();
  return payload.landmarks;
}

export function getPokegearLandmarksSync(): LandmarkEntry[] {
  return loadPokegearPayloadSync().landmarks;
}

export async function getMapToLandmark(): Promise<Record<string, string>> {
  const payload = await loadPayload();
  return payload.map_to_landmark;
}

export function getMapToLandmarkSync(): Record<string, string> {
  return loadPokegearPayloadSync().map_to_landmark;
}
