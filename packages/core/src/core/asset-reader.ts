import fs from "fs";
import { canonicalizeBundledBrowserAssetPath } from "./browser-asset-paths";

const isBrowser = typeof window !== "undefined";
const DEFAULT_CONCURRENCY = 8;
const DEFAULT_FETCH_OPTIONS: RequestInit = { cache: "force-cache" };

type AssetReaderCacheStore = {
  textAssetCache: Map<string, string>;
  jsonAssetCache: Map<string, unknown>;
  inFlightTextAssetRequests: Map<string, Promise<string>>;
  inFlightJsonAssetRequests: Map<string, Promise<unknown>>;
};

const createAssetReaderCacheStore = (): AssetReaderCacheStore => ({
  textAssetCache: new Map<string, string>(),
  jsonAssetCache: new Map<string, unknown>(),
  inFlightTextAssetRequests: new Map<string, Promise<string>>(),
  inFlightJsonAssetRequests: new Map<string, Promise<unknown>>(),
});

const getAssetReaderCacheStore = (): AssetReaderCacheStore => {
  if (!isBrowser && process.env.NODE_ENV === "test") {
    return createAssetReaderCacheStore();
  }
  const scope = globalThis as typeof globalThis & {
    __POKECRYSTAL_ASSET_READER_CACHE__?: AssetReaderCacheStore;
  };
  if (!scope.__POKECRYSTAL_ASSET_READER_CACHE__) {
    scope.__POKECRYSTAL_ASSET_READER_CACHE__ = createAssetReaderCacheStore();
  }
  return scope.__POKECRYSTAL_ASSET_READER_CACHE__;
};

const {
  textAssetCache,
  jsonAssetCache,
  inFlightTextAssetRequests,
  inFlightJsonAssetRequests,
} = getAssetReaderCacheStore();

const invalidateAssetCaches = (path: string): void => {
  textAssetCache.delete(path);
  jsonAssetCache.delete(path);
  inFlightTextAssetRequests.delete(path);
  inFlightJsonAssetRequests.delete(path);
};

const readBrowserTextAssetSyncOnce = (path: string): string => {
  const targetPath = canonicalizeBundledBrowserAssetPath(path);
  if (typeof XMLHttpRequest === "undefined") {
    throw new Error(`XMLHttpRequest unavailable for ${targetPath}`);
  }
  const request = new XMLHttpRequest();
  request.open("GET", targetPath, false);
  try {
    request.send(null);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to load asset ${targetPath}: sync request error: ${message}`);
  }
  if (request.status >= 200 && request.status < 300) {
    return request.responseText ?? "";
  }
  throw new Error(`Failed to load asset ${targetPath} (status ${request.status})`);
};

const readBrowserTextAssetSync = (path: string): string => {
  return readBrowserTextAssetSyncOnce(path);
};

const readBrowserTextAssetOnce = async (path: string): Promise<string> => {
  const targetPath = canonicalizeBundledBrowserAssetPath(path);
  if (typeof fetch !== "function") {
    throw new Error(`fetch unavailable for ${targetPath}`);
  }
  const response = await fetch(targetPath, DEFAULT_FETCH_OPTIONS);
  if (response.ok) {
    return await response.text();
  }
  throw new Error(`Failed to load asset ${targetPath} (status ${response.status})`);
};

const readBrowserTextAsset = async (path: string): Promise<string> => {
  return await readBrowserTextAssetOnce(path);
};

export const readTextAssetSync = (path: string): string => {
  if (!path) {
    throw new Error("readTextAssetSync requires a path.");
  }
  const cached = textAssetCache.get(path);
  if (cached !== undefined) {
    return cached;
  }
  if (!isBrowser) {
    const content = fs.readFileSync(path, "utf-8");
    textAssetCache.set(path, content);
    return content;
  }
  const content = readBrowserTextAssetSync(path);
  textAssetCache.set(path, content);
  return content;
};

export const readJsonAssetSync = <T = unknown>(path: string): T => {
  if (jsonAssetCache.has(path)) {
    return jsonAssetCache.get(path) as T;
  }
  const parseJson = (raw: string): T => JSON.parse(raw) as T;
  try {
    const parsed = parseJson(readTextAssetSync(path));
    jsonAssetCache.set(path, parsed);
    return parsed;
  } catch (error) {
    if (isBrowser) {
      invalidateAssetCaches(path);
      try {
        const parsed = parseJson(readTextAssetSync(path));
        jsonAssetCache.set(path, parsed);
        return parsed;
      } catch (retryError) {
        const message = retryError instanceof Error ? retryError.message : String(retryError);
        throw new Error(`Invalid JSON asset ${path}: ${message}`);
      }
    }
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Invalid JSON asset ${path}: ${message}`);
  }
};

export const readTextAsset = async (path: string): Promise<string> => {
  if (!path) {
    throw new Error("readTextAsset requires a path.");
  }
  const cached = textAssetCache.get(path);
  if (cached !== undefined) {
    return cached;
  }
  const inFlight = inFlightTextAssetRequests.get(path);
  if (inFlight) {
    return inFlight;
  }

  const request = (async (): Promise<string> => {
    if (!isBrowser) {
      const content = await fs.promises.readFile(path, "utf-8");
      textAssetCache.set(path, content);
      return content;
    }
    const content = await readBrowserTextAsset(path);
    textAssetCache.set(path, content);
    return content;
  })();

  inFlightTextAssetRequests.set(path, request);
  try {
    return await request;
  } finally {
    inFlightTextAssetRequests.delete(path);
  }
};

export const readJsonAsset = async <T = unknown>(path: string): Promise<T> => {
  if (jsonAssetCache.has(path)) {
    return jsonAssetCache.get(path) as T;
  }
  const inFlight = inFlightJsonAssetRequests.get(path);
  if (inFlight) {
    return inFlight as Promise<T>;
  }
  const request = (async (): Promise<T> => {
    const parseJson = (raw: string): T => JSON.parse(raw) as T;
    try {
      const parsed = parseJson(await readTextAsset(path));
      jsonAssetCache.set(path, parsed);
      return parsed;
    } catch (error) {
      if (isBrowser) {
        invalidateAssetCaches(path);
        const parsed = parseJson(await readTextAsset(path));
        jsonAssetCache.set(path, parsed);
        return parsed;
      }
      throw error;
    }
  })();
  inFlightJsonAssetRequests.set(path, request);
  try {
    return await request;
  } finally {
    inFlightJsonAssetRequests.delete(path);
  }
};

type PreloadOptions = {
  concurrency?: number;
  onProgress?: (completed: number, total: number, path?: string) => void;
};

export const preloadTextAssets = async (
  paths: string[],
  options: number | PreloadOptions = DEFAULT_CONCURRENCY
): Promise<void> => {
  const uniquePaths = Array.from(new Set(paths.filter(Boolean)));
  if (!uniquePaths.length) {
    return;
  }
  const resolved =
    typeof options === "number" ? { concurrency: options } : options ?? {};
  const limit = Math.max(1, Math.min(resolved.concurrency ?? DEFAULT_CONCURRENCY, uniquePaths.length));
  const total = uniquePaths.length;
  let index = 0;
  let completed = 0;
  resolved.onProgress?.(0, total);
  const worker = async (): Promise<void> => {
    while (index < uniquePaths.length) {
      const current = uniquePaths[index];
      index += 1;
      await readTextAsset(current);
      completed += 1;
      resolved.onProgress?.(completed, total, current);
    }
  };
  await Promise.all(Array.from({ length: limit }, () => worker()));
};
