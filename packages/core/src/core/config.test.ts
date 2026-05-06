import { getSettings, resetSettings } from "./config";

describe("core config playtest MCP session TTL", () => {
  const originalTtl = process.env.POKECRYSTAL_MCP_SESSION_TTL_SECONDS;

  beforeEach(() => {
    resetSettings();
  });

  afterEach(() => {
    if (originalTtl === undefined) {
      delete process.env.POKECRYSTAL_MCP_SESSION_TTL_SECONDS;
    } else {
      process.env.POKECRYSTAL_MCP_SESSION_TTL_SECONDS = originalTtl;
    }
    resetSettings();
  });

  it("clamps configured MCP session TTL to at least 30 minutes", () => {
    process.env.POKECRYSTAL_MCP_SESSION_TTL_SECONDS = "60";
    expect(getSettings().mcpSessionTtlSeconds).toBe(1800);
  });

  it("keeps configured MCP session TTL when above 30 minutes", () => {
    process.env.POKECRYSTAL_MCP_SESSION_TTL_SECONDS = "3600";
    expect(getSettings().mcpSessionTtlSeconds).toBe(3600);
  });
});
