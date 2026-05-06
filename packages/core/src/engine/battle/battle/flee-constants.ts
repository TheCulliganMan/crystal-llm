import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import { joinPath } from "@pokecrystal/core/core/path-utils";

type FleeMonsData = {
  always: string[];
  often: string[];
  sometimes: string[];
};

const getFleeMonsPath = (): string => joinPath(getDataDir(), "flee_mons.json");

const isFleeMonsData = (value: unknown): value is FleeMonsData => {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  const data = value as Partial<Record<keyof FleeMonsData, unknown>>;
  return Array.isArray(data.always) && Array.isArray(data.often) && Array.isArray(data.sometimes);
};

const loadFleeMonsData = (() => {
  let cached: FleeMonsData | null = null;
  return (): FleeMonsData => {
    if (cached) {
      return cached;
    }
    try {
      const fleeMonsPath = getFleeMonsPath();
      const data = readJsonAssetSync<unknown>(fleeMonsPath);
      if (!isFleeMonsData(data)) {
        throw new Error("expected object with always, often, and sometimes arrays");
      }
      cached = data;
    } catch (error) {
      throw new Error(
        `Flee monster tables are required for the asset-only runtime: missing or invalid ${getFleeMonsPath()}.`,
        { cause: error }
      );
    }
    return cached;
  };
})();

const loadFleeSpeciesSet = (key: keyof FleeMonsData): Set<string> => {
  const data = loadFleeMonsData();
  return new Set<string>(data[key]);
};

let alwaysFleeSpecies: Set<string> | null = null;
export const getAlwaysFleeSpecies = (): Set<string> => {
  if (!alwaysFleeSpecies) {
    alwaysFleeSpecies = loadFleeSpeciesSet("always");
  }
  return alwaysFleeSpecies;
};

let oftenFleeSpecies: Set<string> | null = null;
export const getOftenFleeSpecies = (): Set<string> => {
  if (!oftenFleeSpecies) {
    oftenFleeSpecies = loadFleeSpeciesSet("often");
  }
  return oftenFleeSpecies;
};

let sometimesFleeSpecies: Set<string> | null = null;
export const getSometimesFleeSpecies = (): Set<string> => {
  if (!sometimesFleeSpecies) {
    sometimesFleeSpecies = loadFleeSpeciesSet("sometimes");
  }
  return sometimesFleeSpecies;
};
