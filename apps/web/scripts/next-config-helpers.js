const os = require("node:os");

const assetCacheHeaders = [
  {
    key: "Cache-Control",
    value: "public, max-age=31536000, immutable",
  },
];

const createCacheRule = (source) => ({
  source,
  headers: assetCacheHeaders,
});

const parseExplicitAllowedDevOrigins = (env = process.env) =>
  (env.POKECRYSTAL_ALLOWED_DEV_ORIGINS ?? "")
    .split(",")
    .map((origin) => origin.trim())
    .filter(Boolean);

const isExternalIpv4Address = (entry) =>
  entry &&
  entry.internal !== true &&
  entry.family === "IPv4" &&
  typeof entry.address === "string" &&
  entry.address.length > 0;

const resolveAllowedDevOrigins = (
  networkInterfaces = os.networkInterfaces(),
  env = process.env
) => {
  const origins = new Set(parseExplicitAllowedDevOrigins(env));

  for (const entries of Object.values(networkInterfaces)) {
    for (const entry of entries ?? []) {
      if (isExternalIpv4Address(entry)) {
        origins.add(entry.address);
      }
    }
  }

  return Array.from(origins);
};

const resolveStaticCacheRules = (env = process.env) => {
  const rules = [
    createCacheRule("/assets/:path*"),
    createCacheRule("/gfx/:path*"),
    createCacheRule("/index.html"),
    createCacheRule("/(favicon.ico|next.svg|vercel.svg|globe.svg|window.svg|file.svg)"),
  ];

  if (env.NODE_ENV === "production") {
    rules.splice(1, 0, createCacheRule("/_next/static/:path*"));
  }

  return rules;
};

module.exports = { resolveAllowedDevOrigins, resolveStaticCacheRules };
