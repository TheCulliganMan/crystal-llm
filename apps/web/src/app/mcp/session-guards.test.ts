import { createHmac } from "crypto";
import { PRIMARY_MCP_SESSION_ID } from "./session-id";
import {
  clearSessionOwnershipClaims,
  buildSessionSecret,
  isRequestAuthorized,
  normalizeSessionId,
  verifySessionSecret,
} from "./session-guards";

beforeEach(() => {
  clearSessionOwnershipClaims();
});

afterEach(() => {
  clearSessionOwnershipClaims();
});

describe("normalizeSessionId", () => {
  it("uses primary MCP session when empty", () => {
    expect(normalizeSessionId("")).toBe(PRIMARY_MCP_SESSION_ID);
    expect(normalizeSessionId(undefined)).toBe(PRIMARY_MCP_SESSION_ID);
  });

  it("accepts safe identifiers", () => {
    expect(normalizeSessionId("user_123")).toBe("user_123");
    expect(normalizeSessionId("player-abc")).toBe("player-abc");
  });

  it("rejects invalid identifiers", () => {
    expect(() => normalizeSessionId("user/..")).toThrow("Invalid session id.");
    expect(() => normalizeSessionId("tool?name")).toThrow("Invalid session id.");
  });
});

describe("isRequestAuthorized", () => {
  const buildRequest = (headers: Record<string, string> = {}) =>
    new Request("http://localhost", { headers });

  it("allows all requests when no token is configured", () => {
    expect(isRequestAuthorized(buildRequest(), null)).toBe(true);
    expect(isRequestAuthorized(buildRequest(), "")).toBe(true);
  });

  it("rejects requests without a matching token", () => {
    expect(isRequestAuthorized(buildRequest(), "secret")).toBe(false);
    expect(isRequestAuthorized(buildRequest({ authorization: "Bearer nope" }), "secret")).toBe(false);
  });

  it("accepts bearer tokens and header shortcuts", () => {
    expect(isRequestAuthorized(buildRequest({ authorization: "Bearer secret" }), "secret")).toBe(true);
    expect(isRequestAuthorized(buildRequest({ authorization: "secret" }), "secret")).toBe(true);
    expect(isRequestAuthorized(buildRequest({ "x-pokecrystal-token": "secret" }), "secret")).toBe(true);
    expect(isRequestAuthorized(buildRequest({ "x-mcp-token": "secret" }), "secret")).toBe(true);
  });
});

describe("verifySessionSecret", () => {
  const originalRequire = process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
  const originalSessionSecret = process.env.POKECRYSTAL_SESSION_SECRET;
  const originalIdentitySecret = process.env.POKECRYSTAL_IDENTITY_SECRET;
  const originalMcpToken = process.env.POKECRYSTAL_MCP_TOKEN;
  const originalNodeEnv = process.env.NODE_ENV;

  beforeEach(() => {
    process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = "true";
    process.env.POKECRYSTAL_SESSION_SECRET = "test-session-secret-key";
    process.env.POKECRYSTAL_IDENTITY_SECRET = "test-identity-secret-key";
    delete process.env.POKECRYSTAL_MCP_TOKEN;
  });

  afterAll(() => {
    if (originalRequire === undefined) delete process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
    else process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = originalRequire;
    if (originalSessionSecret === undefined) delete process.env.POKECRYSTAL_SESSION_SECRET;
    else process.env.POKECRYSTAL_SESSION_SECRET = originalSessionSecret;
    if (originalIdentitySecret === undefined) delete process.env.POKECRYSTAL_IDENTITY_SECRET;
    else process.env.POKECRYSTAL_IDENTITY_SECRET = originalIdentitySecret;
    if (originalMcpToken === undefined) delete process.env.POKECRYSTAL_MCP_TOKEN;
    else process.env.POKECRYSTAL_MCP_TOKEN = originalMcpToken;
    if (originalNodeEnv === undefined) delete process.env.NODE_ENV;
    else process.env.NODE_ENV = originalNodeEnv;
  });

  it("accepts matching identity-bound session secrets", () => {
    const claims = { v: 1, playerId: "player-1", name: null, iat: 0 };
    const payload = Buffer.from(JSON.stringify(claims), "utf8").toString("base64url");
    const signature = createHmac("sha256", process.env.POKECRYSTAL_IDENTITY_SECRET as string)
      .update(`1.${payload}`)
      .digest("base64url");
    const identityToken = `pcid.1.${payload}.${signature}`;
    const sessionSecret = buildSessionSecret("run-1", "player-1");
    const request = new Request("http://localhost?session_secret=" + encodeURIComponent(sessionSecret), {
      headers: { authorization: `Bearer ${identityToken}` },
    });

    const result = verifySessionSecret(request, "run-1");
    expect(result.ok).toBe(true);
  });

  it("rejects bad secrets when enforcement is enabled", () => {
    const claims = { v: 1, playerId: "player-1", name: null, iat: 0 };
    const payload = Buffer.from(JSON.stringify(claims), "utf8").toString("base64url");
    const signature = createHmac("sha256", process.env.POKECRYSTAL_IDENTITY_SECRET as string)
      .update(`1.${payload}`)
      .digest("base64url");
    const identityToken = `pcid.1.${payload}.${signature}`;
    const request = new Request("http://localhost?session_secret=wrong", {
      headers: { authorization: `Bearer ${identityToken}` },
    });

    const result = verifySessionSecret(request, "run-1");
    expect(result.ok).toBe(false);
    expect(result.status).toBe(401);
  });

  it("rejects a different identity for an already-claimed session id", () => {
    const claimsA = { v: 1, playerId: "player-a", name: null, iat: 0 };
    const payloadA = Buffer.from(JSON.stringify(claimsA), "utf8").toString("base64url");
    const signatureA = createHmac("sha256", process.env.POKECRYSTAL_IDENTITY_SECRET as string)
      .update(`1.${payloadA}`)
      .digest("base64url");
    const identityTokenA = `pcid.1.${payloadA}.${signatureA}`;
    const sessionSecretA = buildSessionSecret("shared-run", "player-a");
    const firstRequest = new Request("http://localhost?session_secret=" + encodeURIComponent(sessionSecretA), {
      headers: { authorization: `Bearer ${identityTokenA}` },
    });
    expect(verifySessionSecret(firstRequest, "shared-run").ok).toBe(true);

    const claimsB = { v: 1, playerId: "player-b", name: null, iat: 0 };
    const payloadB = Buffer.from(JSON.stringify(claimsB), "utf8").toString("base64url");
    const signatureB = createHmac("sha256", process.env.POKECRYSTAL_IDENTITY_SECRET as string)
      .update(`1.${payloadB}`)
      .digest("base64url");
    const identityTokenB = `pcid.1.${payloadB}.${signatureB}`;
    const sessionSecretB = buildSessionSecret("shared-run", "player-b");
    const secondRequest = new Request("http://localhost?session_secret=" + encodeURIComponent(sessionSecretB), {
      headers: { authorization: `Bearer ${identityTokenB}` },
    });

    const result = verifySessionSecret(secondRequest, "shared-run");
    expect(result.ok).toBe(false);
    expect(result.status).toBe(403);
    expect(result.message).toContain("different identity");
  });

  it("throws when production secrets are missing", () => {
    const originalServiceRole = process.env.SUPABASE_SERVICE_ROLE_KEY;
    process.env.NODE_ENV = "production";
    delete process.env.POKECRYSTAL_SESSION_SECRET;
    delete process.env.POKECRYSTAL_IDENTITY_SECRET;
    delete process.env.SUPABASE_SERVICE_ROLE_KEY;

    expect(() => buildSessionSecret("run-1", "player-1")).toThrow(
      "Missing POKECRYSTAL_SESSION_SECRET (or equivalent) in production."
    );

    process.env.NODE_ENV = originalNodeEnv;
    if (originalServiceRole === undefined) {
      delete process.env.SUPABASE_SERVICE_ROLE_KEY;
    } else {
      process.env.SUPABASE_SERVICE_ROLE_KEY = originalServiceRole;
    }
  });
});
