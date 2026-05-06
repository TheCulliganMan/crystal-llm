const {
  resolveAllowedDevOrigins,
  resolveStaticCacheRules,
} = require("../scripts/next-config-helpers");

describe("next config", () => {
  test("allows LAN dev origins so client chunks and HMR load outside localhost", () => {
    const origins = resolveAllowedDevOrigins(
      {
        en0: [
          {
            address: "192.168.50.173",
            family: "IPv4",
            internal: false,
          },
        ],
        lo0: [
          {
            address: "127.0.0.1",
            family: "IPv4",
            internal: true,
          },
        ],
      },
      {}
    );

    expect(origins).toContain("192.168.50.173");
    expect(origins).not.toContain("127.0.0.1");
  });

  test("allows explicit dev origins from env", () => {
    expect(
      resolveAllowedDevOrigins(
        {},
        { POKECRYSTAL_ALLOWED_DEV_ORIGINS: "krabby.local, 10.0.0.42" }
      )
    ).toEqual(["krabby.local", "10.0.0.42"]);
  });

  test("does not mark Next development chunks as immutable", () => {
    const developmentRules = resolveStaticCacheRules({ NODE_ENV: "development" });
    const productionRules = resolveStaticCacheRules({ NODE_ENV: "production" });

    expect(developmentRules.some((rule) => rule.source === "/_next/static/:path*")).toBe(false);
    expect(productionRules.some((rule) => rule.source === "/_next/static/:path*")).toBe(true);
  });
});
