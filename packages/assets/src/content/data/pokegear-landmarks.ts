import path from 'path';
import { z } from 'zod';
import { getDataDir } from '@pokecrystal/core/core/paths';
import { readJsonAssetSync } from '@pokecrystal/core/core/asset-reader';
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

const PokegearLandmarkPayloadSchema = z.object({
  landmarks: z.array(LandmarkEntrySchema),
  map_to_landmark: z.record(z.string(), z.string()),
});

type PokegearLandmarkPayload = z.infer<typeof PokegearLandmarkPayloadSchema>;

export type LandmarkEntry = z.infer<typeof LandmarkEntrySchema>;

function loadPayload(filePath: string): PokegearLandmarkPayload {
  try {
    return PokegearLandmarkPayloadSchema.parse(
      mergePokegearLandmarksPayload(readJsonAssetSync(filePath))
    );
  } catch (error) {
    if (error instanceof Error) {
      throw new Error(`Invalid Pokégear landmark JSON content in ${filePath}: ${error.message}`);
    }
    throw new Error(`Invalid Pokégear landmark JSON content in ${filePath}.`);
  }
}

const payload = loadPayload(path.join(getDataDir(), 'pokegear_landmarks.json'));

export const POKEGEAR_LANDMARKS: LandmarkEntry[] = payload.landmarks;
export const MAP_TO_LANDMARK: Record<string, string> = { ...payload.map_to_landmark };
