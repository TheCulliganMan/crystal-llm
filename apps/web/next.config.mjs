import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const configDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(configDir, "..", "..");
const {
  resolveAllowedDevOrigins,
  resolveStaticCacheRules,
} = require("./scripts/next-config-helpers.js");
const fsShimAbs = path.join(configDir, "src/shims/fs-browser.ts");
const osShimAbs = path.join(configDir, "src/shims/os-browser.ts");
const fsShimRel = "./src/shims/fs-browser.ts";
const osShimRel = "./src/shims/os-browser.ts";
const pathBrowserAbs = require.resolve("path-browserify");
const pathBrowserPkg = "path-browserify";
const coreSrc = path.join(repoRoot, "packages", "core", "src");
const assetsSrc = path.join(repoRoot, "packages", "assets", "src");
const exportersSrc = path.join(repoRoot, "packages", "exporters", "src");
const workspaceAliases = {
  "@pokecrystal/core": coreSrc,
  "@pokecrystal/core/": `${coreSrc}/`,
  "@pokecrystal/assets": assetsSrc,
  "@pokecrystal/assets/": `${assetsSrc}/`,
  "@pokecrystal/exporters": exportersSrc,
  "@pokecrystal/exporters/": `${exportersSrc}/`,
};

const clientAliases = {
  fs: fsShimAbs,
  "fs/promises": fsShimAbs,
  "node:fs": fsShimAbs,
  "node:fs/promises": fsShimAbs,
  path: pathBrowserAbs,
  "node:path": pathBrowserAbs,
  os: osShimAbs,
  "node:os": osShimAbs,
};

const turboAliases = {
  fs: { browser: fsShimRel, node: "fs" },
  "fs/promises": { browser: fsShimRel, node: "fs/promises" },
  "node:fs": { browser: fsShimRel, node: "node:fs" },
  "node:fs/promises": { browser: fsShimRel, node: "node:fs/promises" },
  path: { browser: pathBrowserPkg, node: "path" },
  "node:path": { browser: pathBrowserPkg, node: "node:path" },
  os: { browser: osShimRel, node: "os" },
  "node:os": { browser: osShimRel, node: "node:os" },
};

const tracingAssetGlobs = [
  "./assets/**/*",
];

const resolveNextOutput = () => {
  const output = process.env.POKECRYSTAL_NEXT_OUTPUT?.trim();
  return output ? output : undefined;
};

/** @type {import('next').NextConfig} */
const config = {
  distDir: process.env.POKECRYSTAL_NEXT_DIST_DIR || ".next",
  output: resolveNextOutput(),
  allowedDevOrigins: resolveAllowedDevOrigins(),
  transpilePackages: ["@pokecrystal/core", "@pokecrystal/assets", "@pokecrystal/exporters"],
  typescript: {
    ignoreBuildErrors: false,
  },
  outputFileTracingIncludes: {
    "/*": tracingAssetGlobs,
  },
  async headers() {
    return resolveStaticCacheRules();
  },
  webpack: (cfg, { isServer, webpack }) => {
    cfg.resolve.alias = {
      ...(cfg.resolve.alias || {}),
      ...workspaceAliases,
      ...(isServer ? {} : clientAliases),
    };
    if (!isServer) {
      cfg.resolve.fallback = {
        ...(cfg.resolve.fallback || {}),
        ...clientAliases,
      };
      cfg.plugins = [
        ...(cfg.plugins || []),
        new webpack.NormalModuleReplacementPlugin(/^node:fs(?:\/promises)?$/, fsShimAbs),
        new webpack.NormalModuleReplacementPlugin(/^node:path$/, pathBrowserAbs),
        new webpack.NormalModuleReplacementPlugin(/^node:os$/, osShimAbs),
      ];
    }
    return cfg;
  },
};

export default config;
