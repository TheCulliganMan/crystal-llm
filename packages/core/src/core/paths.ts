import fs from "fs";
import path from "path";
import { joinPath, normalizePath } from "./path-utils";

let basePath: string | null = null;
let assetsRoot: string | null = null;
let disassemblyRoot: string | null = null;
const isBrowser = typeof window !== "undefined";
const WEB_ASSETS_ROOT = process.env.NEXT_PUBLIC_ASSET_BASE || "/assets";
const WEB_APP_SUFFIX = "/apps/web";
const ASSET_SENTINEL_PARTS = ["data", "map_attributes.json"];
const ASSET_GRAPHICS_SENTINEL_PARTS = ["gfx", "tilesets", "bg_tiles.pal"];
const DISASSEMBLY_SENTINEL_PARTS = ["engine", "events", "specials.asm"];
const DISASSEMBLY_ROOT_CANDIDATES = [
  ["vendor", "pokecrystal"],
  ["pokecrystal_disassembly"],
] as const;

const resolveWorkspaceRoot = (cwd: string): string => {
  const workspaceIndex = cwd.lastIndexOf(WEB_APP_SUFFIX);
  if (workspaceIndex !== -1) {
    return cwd.slice(0, workspaceIndex + WEB_APP_SUFFIX.length) || "/";
  }

  let current = normalizePath(cwd || "/");
  for (let depth = 0; depth < 12; depth += 1) {
    const directWorkspace = joinPath(current, "package.json");
    if (
      path.basename(current) === "web" &&
      path.basename(path.dirname(current)) === "apps" &&
      fs.existsSync(directWorkspace)
    ) {
      return current;
    }

    const nestedWorkspace = joinPath(current, "apps", "web", "package.json");
    if (fs.existsSync(nestedWorkspace)) {
      return joinPath(current, "apps", "web");
    }

    const parent = normalizePath(path.dirname(current));
    if (parent === current) {
      break;
    }
    current = parent;
  }

  return joinPath(cwd, "apps", "web");
};

const resolveRepoRoot = (workspaceRoot: string): string => {
  const workspaceIndex = workspaceRoot.lastIndexOf(WEB_APP_SUFFIX);
  if (workspaceIndex !== -1) {
    return workspaceRoot.slice(0, workspaceIndex) || "/";
  }

  const workspaceParent = normalizePath(path.dirname(workspaceRoot));
  if (path.basename(workspaceParent) === "apps") {
    return normalizePath(path.dirname(workspaceParent));
  }

  return workspaceParent;
};

const resolveExistingRoot = (candidates: string[], sentinelParts?: string[]): string | null => {
  for (const candidate of candidates) {
    if (!candidate || !fs.existsSync(candidate)) {
      continue;
    }
    if (sentinelParts?.length) {
      const sentinelPath = joinPath(candidate, ...sentinelParts);
      if (!fs.existsSync(sentinelPath)) {
        continue;
      }
    }
    return candidate;
  }
  return null;
};

const findAssetsRootFromCwd = (cwd: string): string | null => {
  let current = normalizePath(cwd || "/");
  for (let depth = 0; depth < 12; depth += 1) {
    const assetsCandidate = joinPath(current, "assets");
    if (
      fs.existsSync(joinPath(assetsCandidate, ...ASSET_SENTINEL_PARTS)) &&
      fs.existsSync(joinPath(assetsCandidate, ...ASSET_GRAPHICS_SENTINEL_PARTS))
    ) {
      return assetsCandidate;
    }
    const parent = normalizePath(path.dirname(current));
    if (parent === current) {
      break;
    }
    current = parent;
  }
  return null;
};

const findDisassemblyRootFromCwd = (cwd: string): string | null => {
  let current = normalizePath(cwd || "/");
  for (let depth = 0; depth < 12; depth += 1) {
    for (const candidateParts of DISASSEMBLY_ROOT_CANDIDATES) {
      const disassemblyCandidate = joinPath(current, ...candidateParts);
      if (fs.existsSync(joinPath(disassemblyCandidate, ...DISASSEMBLY_SENTINEL_PARTS))) {
        return disassemblyCandidate;
      }
    }
    const parent = normalizePath(path.dirname(current));
    if (parent === current) {
      break;
    }
    current = parent;
  }
  return null;
};

export function getBasePath(): string {
  if (basePath) {
    return basePath;
  }

  if (isBrowser) {
    basePath = WEB_ASSETS_ROOT;
    return basePath;
  }
  basePath = resolveWorkspaceRoot(normalizePath(process.cwd()));
  return basePath;
}

export function getDisassemblyRoot(): string {
  if (isBrowser) {
    return normalizePath(process.env.POKECRYSTAL_DISASSEMBLY_ROOT || "/vendor/pokecrystal");
  }
  if (disassemblyRoot) {
    return disassemblyRoot;
  }
  const override = process.env.POKECRYSTAL_DISASSEMBLY_ROOT;
  if (override) {
    disassemblyRoot = normalizePath(override);
    return disassemblyRoot;
  }
  const workspaceRoot = getBasePath();
  const repoRoot = resolveRepoRoot(workspaceRoot);
  const resolved = resolveExistingRoot(
    DISASSEMBLY_ROOT_CANDIDATES.map((candidateParts) => joinPath(repoRoot, ...candidateParts)),
    DISASSEMBLY_SENTINEL_PARTS
  );
  const fallback = resolved ?? findDisassemblyRootFromCwd(process.cwd());
  if (fallback) {
    disassemblyRoot = fallback;
    return disassemblyRoot;
  }
  return joinPath(repoRoot, "vendor", "pokecrystal");
}

export function getDataDir(): string {
  return joinPath(getAssetsRoot(), "data");
}

export function getAssetsRoot(): string {
  if (isBrowser) {
    return WEB_ASSETS_ROOT;
  }
  if (assetsRoot) {
    return assetsRoot;
  }
  const workspaceRoot = getBasePath();
  const repoRoot = resolveRepoRoot(workspaceRoot);
  const resolved = resolveExistingRoot(
    [
      joinPath(workspaceRoot, "assets"),
    ],
    ASSET_SENTINEL_PARTS
  );
  const withGraphics =
    resolved && fs.existsSync(joinPath(resolved, ...ASSET_GRAPHICS_SENTINEL_PARTS))
      ? resolved
      : null;
  const fallback = withGraphics ?? findAssetsRootFromCwd(process.cwd());
  assetsRoot = fallback ?? joinPath(workspaceRoot, "assets");
  return assetsRoot;
}

export function getAssetPath(...parts: string[]): string {
  return joinPath(getAssetsRoot(), ...parts);
}

export function getTilesetCollisionPath(tilesetName: string): string {
  return joinPath(getDataDir(), "tilesets", `${tilesetName}.json`);
}

export function getTilesetPaletteMapJsonPath(tilesetName: string): string {
  return joinPath(getDataDir(), "tilesets", `${tilesetName}_palette_map.json`);
}

export function getTilesetMetatilesPath(tilesetName: string): string {
  return joinPath(
    getDataDir(),
    "tilesets",
    `${tilesetName}_metatiles.bin`
  );
}

export function getAnimationsOutputPath(): string {
  return joinPath(getDataDir(), "animations.json");
}
