import fs from "fs";
import manifest from "../../assets.manifest.json";
import { canonicalizeBundledBrowserAssetPath } from "./browser-asset-paths";
import { normalizePath } from "./path-utils";
import { getAssetsRoot, getDisassemblyRoot } from "./paths";

type Manifest = {
  files: string[];
  directories: Record<string, string[]>;
};

const normalizeManifestPath = (value: string): string => {
  const normalized = normalizePath(value);
  if (normalized.length > 1) {
    return normalized.replace(/\/+$/, "");
  }
  return normalized;
};

const typedManifest = manifest as Manifest;
const manifestFileEntries = typedManifest.files
  .map((entry) => normalizeManifestPath(entry))
  .filter((entry) => !entry.startsWith("/disassembly/"));
const manifestFiles = new Set(
  manifestFileEntries
);
const manifestDirs = new Map(
  Object.entries(typedManifest.directories)
    .filter(([dir]) => {
      const normalized = normalizeManifestPath(dir);
      return normalized !== "/disassembly" && !normalized.startsWith("/disassembly/");
    })
    .map(([dir, entries]) => [
      normalizeManifestPath(dir),
      entries.filter((entry) => entry !== "disassembly"),
    ])
);

const manifestContainsFile = (manifestPath: string): boolean => {
  if (manifestFiles.has(manifestPath)) {
    return true;
  }
  const normalized = normalizeManifestPath(manifestPath);
  const parent = normalizeManifestPath(normalized.slice(0, normalized.lastIndexOf("/")) || "/");
  const base = normalized.slice(normalized.lastIndexOf("/") + 1);
  const entries = manifestDirs.get(parent);
  return Boolean(entries?.includes(base));
};

const probeBrowserAssetExists = (manifestPath: string): boolean => {
  if (typeof window === "undefined" || typeof XMLHttpRequest === "undefined") {
    return false;
  }
  try {
    const xhr = new XMLHttpRequest();
    xhr.open("HEAD", toManifestPath(manifestPath), false);
    xhr.send(null);
    return xhr.status >= 200 && xhr.status < 400;
  } catch {
    return false;
  }
};

const assetDiskRoot = normalizeManifestPath(getAssetsRoot());
const disassemblyDiskRoot = normalizeManifestPath(getDisassemblyRoot());
const isNodeLocalPath = (value: string): boolean => {
  const normalized = normalizeManifestPath(value);
  return (
    Boolean(assetDiskRoot && normalized.startsWith(`${assetDiskRoot}/`)) ||
    Boolean(disassemblyDiskRoot && normalized.startsWith(`${disassemblyDiskRoot}/`)) ||
    fs.existsSync(normalized)
  );
};

const toManifestPath = (value: string): string => {
  const normalized = normalizeManifestPath(
    canonicalizeBundledBrowserAssetPath(value)
  );
  if (assetDiskRoot && normalized.startsWith(`${assetDiskRoot}/`)) {
    return normalizeManifestPath(
      `/assets/${normalized.slice(assetDiskRoot.length + 1)}`
    );
  }
  if (normalized.startsWith("/")) {
    return normalized;
  }
  return `/${normalized}`;
};

const isManifestManagedPath = (value: string): boolean =>
  value === "/assets" ||
  (value.startsWith("/assets/") && !value.startsWith("/assets/data/"));

export const toPublicAssetUrl = (filePath: string): string => toManifestPath(filePath);

export const assetExists = (filePath: string): boolean => {
  if (typeof window === "undefined" && isNodeLocalPath(filePath)) {
    return fs.existsSync(normalizeManifestPath(filePath));
  }
  const manifestPath = toManifestPath(filePath);
  if (manifestContainsFile(manifestPath) || manifestDirs.has(manifestPath)) {
    return true;
  }
  if (isManifestManagedPath(manifestPath)) {
    return false;
  }
  return probeBrowserAssetExists(manifestPath);
};

export const listAssetDir = (dirPath: string): string[] => {
  if (typeof window === "undefined" && isNodeLocalPath(dirPath)) {
    try {
      return fs.readdirSync(normalizeManifestPath(dirPath)).sort((a, b) => a.localeCompare(b));
    } catch {
      return [];
    }
  }
  const manifestPath = toManifestPath(dirPath);
  return manifestDirs.get(manifestPath) ?? [];
};

export const listAssetFilesBySuffixes = (
  suffixes: string[],
  options?: { prefixes?: string[] }
): string[] => {
  const normalizedSuffixes = suffixes.map((suffix) => suffix.toLowerCase());
  const prefixes = (options?.prefixes ?? []).map((prefix) =>
    normalizeManifestPath(prefix).toLowerCase()
  );
  return manifestFileEntries.filter((file) => {
    const normalized = normalizeManifestPath(file);
    const lowered = normalized.toLowerCase();
    if (prefixes.length && !prefixes.some((prefix) => lowered.startsWith(prefix))) {
      return false;
    }
    return normalizedSuffixes.some((suffix) => lowered.endsWith(suffix));
  });
};
