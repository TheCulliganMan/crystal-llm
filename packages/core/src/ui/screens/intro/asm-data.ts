import path from "path";
import { z } from "zod";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";

const OAMPieceSchema = z.object({
  x: z.number(),
  y: z.number(),
  tile: z.number(),
  attributes: z.number(),
});
export type OAMPiece = z.infer<typeof OAMPieceSchema>;

const SpriteOAMSetSchema = z.object({
  name: z.string(),
  tile_offset: z.number(),
  pieces: z.array(OAMPieceSchema).default([]),
});
export type SpriteOAMSet = z.infer<typeof SpriteOAMSetSchema>;

const FrameStepSchema = z.object({
  oam_set: z.string().nullable(),
  duration: z.number(),
  attr_flags: z.number().default(0),
  command: z.enum(["frame", "delete", "restart", "end", "wait"]).default("frame"),
});
export type FrameStep = z.infer<typeof FrameStepSchema>;

const FramesetSchema = z.object({
  name: z.string(),
  steps: z.array(FrameStepSchema).default([]),
});
export type Frameset = z.infer<typeof FramesetSchema>;

const SpriteObjectDefinitionSchema = z.object({
  name: z.string(),
  frameset: z.string(),
  function: z.string(),
  dictionary: z.string(),
});
export type SpriteObjectDefinition = z.infer<typeof SpriteObjectDefinitionSchema>;

const SpriteAnimRuntimeBundleSchema = z.object({
  oam_sets: z.record(z.string(), SpriteOAMSetSchema).default({}),
  framesets: z.record(z.string(), FramesetSchema).default({}),
  objects: z.record(z.string(), SpriteObjectDefinitionSchema).default({}),
});

type SpriteAnimRuntimeBundle = z.infer<typeof SpriteAnimRuntimeBundleSchema>;

const SPRITE_ANIM_BUNDLE_PATH = path.join(getDataDir(), "sprite_anim_bundle.json");

let runtimeBundleCache: SpriteAnimRuntimeBundle | null = null;

function loadRuntimeBundle(): SpriteAnimRuntimeBundle {
  if (runtimeBundleCache) {
    return runtimeBundleCache;
  }
  let parsed: unknown;
  try {
    parsed = readJsonAssetSync<unknown>(SPRITE_ANIM_BUNDLE_PATH);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Missing bundled sprite animation runtime file: ${SPRITE_ANIM_BUNDLE_PATH}. ${message}`);
  }
  runtimeBundleCache = SpriteAnimRuntimeBundleSchema.parse(parsed);
  return runtimeBundleCache;
}

export function loadSpriteOamSets(): Record<string, SpriteOAMSet> {
  return loadRuntimeBundle().oam_sets;
}

export function loadFramesets(): Record<string, Frameset> {
  return loadRuntimeBundle().framesets;
}

export function loadSpriteObjectDefinitions(): Record<string, SpriteObjectDefinition> {
  return loadRuntimeBundle().objects;
}

export function fetchOamSet(name: string): SpriteOAMSet {
  const sets = loadSpriteOamSets();
  if (!sets[name]) {
    throw new Error(`OAM set ${name} not found in parsed data`);
  }
  return sets[name];
}

export function fetchFrameset(name: string): Frameset {
  const framesets = loadFramesets();
  if (!framesets[name]) {
    throw new Error(`Frameset ${name} not found`);
  }
  return framesets[name];
}

export function fetchSpriteObject(name: string): SpriteObjectDefinition {
  const objects = loadSpriteObjectDefinitions();
  if (!objects[name]) {
    throw new Error(`Sprite object ${name} not found`);
  }
  return objects[name];
}
