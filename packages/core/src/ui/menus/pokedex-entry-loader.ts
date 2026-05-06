import fs from "fs";
import path from "path";
import { getDataDir } from "../../core/paths";

export type PokedexEntryData = {
  classification: string;
  heightDigits: number;
  weightDigits: number;
  pages: string[];
};

type PokedexPayloadEntry = {
  species: string;
  classification: string;
  heightDigits: number;
  weightDigits: number;
  pages: string[];
};

const POKEDEX_JSON_PATH = path.join(getDataDir(), "pokedex_entries.json");

let pokedexEntryCache: Map<string, PokedexPayloadEntry> | null = null;

const normalizeSpeciesKey = (value: string): string =>
  String(value)
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");

const loadPokedexEntries = (): Map<string, PokedexPayloadEntry> => {
  if (pokedexEntryCache) {
    return pokedexEntryCache;
  }
  const raw = fs.readFileSync(POKEDEX_JSON_PATH, "utf-8");
  const payload = JSON.parse(raw) as PokedexPayloadEntry[];
  pokedexEntryCache = new Map(
    payload.map((entry) => [normalizeSpeciesKey(String(entry.species)), entry])
  );
  return pokedexEntryCache;
};

export const parsePokedexEntryFile = (speciesId: string): PokedexEntryData => {
  const normalizedSpecies = normalizeSpeciesKey(speciesId);
  const entry = loadPokedexEntries().get(normalizedSpecies);
  if (!entry) {
    throw new Error(
      `Missing Pok\u00e9dex entry definition for ${speciesId} (${POKEDEX_JSON_PATH}).`
    );
  }
  const pages = entry.pages;
  if (!pages.length) {
    throw new Error(`Pok\u00e9dex entry text missing for ${speciesId}.`);
  }
  if (!entry.classification) {
    throw new Error(`Missing classification text for ${speciesId}.`);
  }
  const heightDigits = entry.heightDigits;
  const weightDigits = entry.weightDigits;
  if (!Number.isFinite(heightDigits) || !Number.isFinite(weightDigits)) {
    throw new Error(`Missing height/weight data for ${speciesId}.`);
  }
  return {
    classification: entry.classification,
    heightDigits,
    weightDigits,
    pages,
  };
};
