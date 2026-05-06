import { Buffer } from "buffer";
import { canonicalizeBundledBrowserAssetPath } from "@pokecrystal/core/core/browser-asset-paths";
import manifest from "../../assets.manifest.json";

type Encoding = BufferEncoding | null | undefined;

const ASSET_BASE = process.env.NEXT_PUBLIC_ASSET_BASE || "/assets";
const DISASSEMBLY_BASE = process.env.NEXT_PUBLIC_DISASSEMBLY_BASE || "/disassembly";
const fileCache = new Map<string, Buffer>();
const inflightReads = new Map<string, Promise<Buffer>>();
const CACHE_NAME = "pokecrystal-fs-cache-v1";

function normalize(p: string): string {
  const replaced = p.replace(/\\/g, "/");
  const protocolMatch = replaced.match(/^([a-zA-Z][a-zA-Z\d+\-.]*):\/\//);
  if (protocolMatch) {
    const protocol = protocolMatch[1];
    const rest = replaced.slice(protocol.length + 3);
    return `${protocol}://${rest.replace(/\/+/g, "/")}`;
  }
  return replaced.replace(/\/+/g, "/");
}

const normalizeManifestPath = (value: string): string => {
  const normalized = normalize(value);
  if (normalized.length > 1) {
    return normalized.replace(/\/+$/, "");
  }
  return normalized;
};

const ASSET_BASE_NORMALIZED = normalizeManifestPath(ASSET_BASE);
const DISASSEMBLY_BASE_NORMALIZED = normalizeManifestPath(DISASSEMBLY_BASE);
const manifestFiles = new Set(
  manifest.files
    .map((entry) => normalizeManifestPath(entry))
    .filter((entry) => !entry.startsWith("/disassembly/"))
);
const manifestDirs: Record<string, string[]> = {};
for (const [dir, entries] of Object.entries(manifest.directories)) {
  const normalizedDir = normalizeManifestPath(dir);
  if (normalizedDir === "/disassembly" || normalizedDir.startsWith("/disassembly/")) {
    continue;
  }
  manifestDirs[normalizedDir] = entries.filter((entry) => entry !== "disassembly");
}

function manifestContainsFile(filePath: string): boolean {
  const normalized = normalizeManifestPath(filePath);
  const normalizedLower = normalized.toLowerCase();
  if (manifestFiles.has(normalized) || manifestFiles.has(normalizedLower)) {
    return true;
  }
  const slash = normalized.lastIndexOf("/");
  const parent = normalizeManifestPath(slash > 0 ? normalized.slice(0, slash) : "/");
  const parentLower = parent.toLowerCase();
  const base = normalized.slice(slash + 1);
  const entries = manifestDirs[parent] ?? manifestDirs[parentLower];
  if (!entries) {
    return false;
  }
  return entries.includes(base) || entries.includes(base.toLowerCase());
}

function toUrl(filePath: string): string {
  const normalized = normalizeManifestPath(
    canonicalizeBundledBrowserAssetPath(filePath, {
      assetBase: ASSET_BASE_NORMALIZED,
      disassemblyBase: DISASSEMBLY_BASE_NORMALIZED,
    }),
  );
  if (
    normalized.startsWith(`${DISASSEMBLY_BASE_NORMALIZED}/`) ||
    normalized === DISASSEMBLY_BASE_NORMALIZED
  ) {
    return normalized;
  }
  if (
    normalized.startsWith(`${ASSET_BASE_NORMALIZED}/`) ||
    normalized === ASSET_BASE_NORMALIZED
  ) {
    return normalized;
  }
  const assetIdx = normalized.lastIndexOf("/assets/");
  if (assetIdx !== -1) {
    return `${ASSET_BASE_NORMALIZED}${normalized.substring(assetIdx + "/assets".length)}`;
  }
  const disasmIdx = normalized.lastIndexOf("/pokecrystal_disassembly/");
  if (disasmIdx !== -1) {
    return `${DISASSEMBLY_BASE_NORMALIZED}${normalized.substring(
      disasmIdx + "/pokecrystal_disassembly".length
    )}`;
  }
  const disassemblyIdx = normalized.lastIndexOf("/disassembly/");
  if (disassemblyIdx !== -1) {
    return `${DISASSEMBLY_BASE_NORMALIZED}${normalized.substring(
      disassemblyIdx + "/disassembly".length
    )}`;
  }
  // Fallback: treat as relative to asset base.
  return `${ASSET_BASE_NORMALIZED}/${normalized.replace(/^\/+/, "")}`;
}

function cacheKey(filePath: string): string {
  return toUrl(filePath);
}

function legacyBinarySibling(url: string): string | null {
  if (url.endsWith(".2bpp.lz")) {
    return url.replace(/\.2bpp\.lz$/, ".2bpp");
  }
  if (url.endsWith(".1bpp.lz")) {
    return url.replace(/\.1bpp\.lz$/, ".1bpp");
  }
  return null;
}

function candidateUrls(filePath: string): string[] {
  const primary = cacheKey(filePath);
  const fallback = legacyBinarySibling(primary);
  return Array.from(new Set([primary, fallback].filter((value): value is string => Boolean(value))));
}

function isManifestManagedPath(filePath: string): boolean {
  const url = toUrl(filePath);
  return (
    url === ASSET_BASE_NORMALIZED ||
    (url.startsWith(`${ASSET_BASE_NORMALIZED}/`) &&
      !url.startsWith(`${ASSET_BASE_NORMALIZED}/data/`))
  );
}

function fetchBinarySyncOnce(url: string): Buffer {
  if (typeof XMLHttpRequest === "undefined") {
    throw new Error("Synchronous asset fetch is only available in the browser.");
  }
  const xhr = new XMLHttpRequest();
  xhr.open("GET", url, false);
  try {
    // Sync XHR from a document cannot set responseType; use text fallback.
    if (typeof xhr.overrideMimeType === "function") {
      xhr.overrideMimeType("text/plain; charset=x-user-defined");
    }
    xhr.send(null);
    if (xhr.status >= 200 && xhr.status < 300) {
      if (xhr.response) {
        if (xhr.response instanceof ArrayBuffer) {
          return Buffer.from(xhr.response);
        }
        if (typeof xhr.response === "string") {
          const bytes = new Uint8Array(xhr.response.length);
          for (let i = 0; i < xhr.response.length; i += 1) {
            bytes[i] = xhr.response.charCodeAt(i) & 0xff;
          }
          return Buffer.from(bytes);
        }
      }
      if (xhr.responseText) {
        const bytes = new Uint8Array(xhr.responseText.length);
        for (let i = 0; i < xhr.responseText.length; i += 1) {
          bytes[i] = xhr.responseText.charCodeAt(i) & 0xff;
        }
        return Buffer.from(bytes);
      }
    }
  } catch (error) {
    // Ignore and throw below.
  }
  const message = xhr.status
    ? `Failed to load ${url} (${xhr.status})`
    : `Unable to read asset ${url}`;
  const err = new Error(message);
  if (xhr.status === 404) {
    (err as NodeJS.ErrnoException).code = "ENOENT";
  }
  throw err;
}

function fetchBinarySync(url: string): Buffer {
  return fetchBinarySyncOnce(url);
}

async function fetchBinaryOnce(url: string): Promise<Buffer> {
  if (typeof caches !== "undefined") {
    try {
      const cache = await caches.open(CACHE_NAME);
      const cached = await cache.match(url);
      if (cached) {
        const buffer = await cached.arrayBuffer();
        return Buffer.from(buffer);
      }
    } catch {
      // Cache API is optional; fall back to network.
    }
  }
  const response = await fetch(url, { cache: "force-cache" });
  if (!response.ok) {
    const err = new Error(`Failed to load ${url} (${response.status})`);
    if (response.status === 404) {
      (err as NodeJS.ErrnoException).code = "ENOENT";
    }
    throw err;
  }
  const buffer = await response.arrayBuffer();
  if (typeof caches !== "undefined") {
    try {
      const cache = await caches.open(CACHE_NAME);
      await cache.put(url, response.clone());
    } catch {
      // Ignore cache failures; the in-memory cache remains authoritative.
    }
  }
  return Buffer.from(buffer);
}

async function fetchBinary(url: string): Promise<Buffer> {
  return await fetchBinaryOnce(url);
}

function headExistsSync(url: string): boolean {
  if (typeof XMLHttpRequest === "undefined") {
    return false;
  }
  const xhr = new XMLHttpRequest();
  xhr.open("HEAD", url, false);
  xhr.send(null);
  return xhr.status >= 200 && xhr.status < 400;
}

function readFileSync(filePath: string, encoding?: Encoding): string | Buffer {
  const keys = candidateUrls(filePath);
  const requestedKey = keys[0];
  let data: Buffer | undefined;
  let resolvedKey = requestedKey;
  let lastError: unknown;

  for (const key of keys) {
    const cached = fileCache.get(key);
    if (cached) {
      data = cached;
      resolvedKey = key;
      break;
    }
    try {
      data = fetchBinarySync(key);
      resolvedKey = key;
      break;
    } catch (error) {
      lastError = error;
    }
  }

  if (!data) {
    throw lastError;
  }

  fileCache.set(requestedKey, data);
  fileCache.set(resolvedKey, data);
  if (encoding) {
    return data.toString(encoding as BufferEncoding);
  }
  return data;
}

async function readFile(
  filePath: string,
  encoding?: Encoding
): Promise<string | Buffer> {
  const keys = candidateUrls(filePath);
  const requestedKey = keys[0];

  for (const key of keys) {
    const cached = fileCache.get(key);
    if (cached) {
      if (key !== requestedKey) {
        fileCache.set(requestedKey, cached);
      }
      return encoding ? cached.toString(encoding as BufferEncoding) : cached;
    }
  }

  const inFlight = inflightReads.get(requestedKey);
  const pending = inFlight ?? (async (): Promise<Buffer> => {
    let lastError: unknown;
    for (const key of keys) {
      try {
        const data = await fetchBinary(key);
        fileCache.set(requestedKey, data);
        fileCache.set(key, data);
        return data;
      } catch (error) {
        lastError = error;
      }
    }
    throw lastError;
  })().finally(() => {
    inflightReads.delete(requestedKey);
  });
  if (!inFlight) {
    inflightReads.set(requestedKey, pending);
  }
  const data = await pending;
  if (encoding) {
    return data.toString(encoding as BufferEncoding);
  }
  return data;
}

async function prefetchFiles(
  filePaths: string[],
  options: {
    ignoreMissing?: boolean;
    concurrency?: number;
    onProgress?: (completed: number, total: number, path?: string) => void;
  } = {}
): Promise<void> {
  const uniquePaths = Array.from(new Set(filePaths.map((path) => cacheKey(path))));
  const concurrency = Math.max(1, options.concurrency ?? 24);
  const total = uniquePaths.length;
  let index = 0;
  let completed = 0;
  options.onProgress?.(0, total);
  const runBatch = async (): Promise<void> => {
    while (index < uniquePaths.length) {
      const current = index;
      index += 1;
      const path = uniquePaths[current];
      try {
        await readFile(path);
      } catch (error) {
        const err = error as NodeJS.ErrnoException;
        if (options.ignoreMissing && err?.code === "ENOENT") {
          completed += 1;
          options.onProgress?.(completed, total, path);
          continue;
        }
        throw error;
      }
      completed += 1;
      options.onProgress?.(completed, total, path);
    }
  };
  const workers = Array.from({ length: Math.min(concurrency, uniquePaths.length) }, () => runBatch());
  await Promise.all(workers);
}

async function writeFile(filePath: string, data: string | Uint8Array): Promise<void> {
  writeFileSync(filePath, data);
}

async function mkdir(): Promise<void> {
  mkdirSync();
}

async function unlink(filePath: string): Promise<void> {
  if (typeof localStorage !== "undefined") {
    localStorage.removeItem(`fs:${normalize(filePath)}`);
  }
}

async function access(filePath: string): Promise<void> {
  if (existsSync(filePath)) {
    return;
  }
  const error = new Error(`ENOENT: no such file or directory, access '${filePath}'`) as NodeJS.ErrnoException;
  error.code = "ENOENT";
  throw error;
}

function existsSync(filePath: string): boolean {
  const normalized = normalizeManifestPath(filePath);
  const normalizedLower = normalized.toLowerCase();
  if (
    manifestContainsFile(normalized) ||
    manifestDirs[normalized] ||
    manifestDirs[normalizedLower]
  ) {
    return true;
  }
  if (isManifestManagedPath(filePath)) {
    return candidateUrls(filePath).some((url) => manifestContainsFile(url));
  }
  try {
    for (const url of candidateUrls(filePath)) {
      if (headExistsSync(url)) {
        return true;
      }
    }
    return false;
  } catch {
    return false;
  }
}

function readdirSync(dirPath: string): string[] {
  const normalized = normalizeManifestPath(dirPath);
  return manifestDirs[normalized] ?? [];
}

function mkdirSync(): void {
  // No-op for browser shim; persistent writes not supported.
}

function writeFileSync(filePath: string, data: string | Uint8Array): void {
  if (typeof localStorage === "undefined") {
    return;
  }
  const key = `fs:${normalize(filePath)}`;
  if (typeof data === "string") {
    localStorage.setItem(key, data);
  } else {
    localStorage.setItem(key, Buffer.from(data).toString("base64"));
  }
}

function statSync(filePath: string): {
  size: number;
  isFile: () => boolean;
  isDirectory: () => boolean;
} {
  const normalized = normalizeManifestPath(filePath);
  const isDir = Boolean(manifestDirs[normalized]);
  return {
    size: 0,
    isFile: () => !isDir,
    isDirectory: () => isDir,
  };
}

const promises = {
  access,
  readFile,
  writeFile,
  mkdir,
  unlink,
};

const fsBrowser = {
  existsSync,
  readFileSync,
  readdirSync,
  mkdirSync,
  writeFileSync,
  statSync,
  promises,
  prefetchFiles,
};

export default fsBrowser;

export {
  existsSync,
  readFileSync,
  readdirSync,
  mkdirSync,
  writeFileSync,
  statSync,
  readFile,
  writeFile,
  mkdir,
  unlink,
  promises,
  prefetchFiles,
};
