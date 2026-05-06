import { buildAuthCallbackUrl, buildRecoveryRedirect, resolvePostAuthRedirect, sanitizeNextPath } from "./urls";

describe("supabase url helpers", () => {
  it("sanitizes next paths to guard against open redirects", () => {
    expect(sanitizeNextPath(undefined)).toBeNull();
    expect(sanitizeNextPath("")).toBeNull();
    expect(sanitizeNextPath("https://example.com")).toBeNull();
    expect(sanitizeNextPath("//evil.com")).toBeNull();
    expect(sanitizeNextPath("ftp://evil.com")).toBeNull();
    expect(sanitizeNextPath("/arena?mode=ranked")).toBe("/arena?mode=ranked");
  });

  it("builds auth callback URLs with optional next parameters", () => {
    const url = buildAuthCallbackUrl("https://arena.example.com", "/arena");
    expect(url).toBe("https://arena.example.com/auth/callback?next=%2Farena");
    const fallbackUrl = buildAuthCallbackUrl("https://arena.example.com", "https://evil.com");
    expect(fallbackUrl).toBe("https://arena.example.com/auth/callback");
  });

  it("resolves post-auth redirects safely", () => {
    const resolved = resolvePostAuthRedirect("https://arena.example.com", "/watch?run=1");
    expect(resolved.toString()).toBe("https://arena.example.com/watch?run=1");
    const fallback = resolvePostAuthRedirect("https://arena.example.com", "https://evil.com");
    expect(fallback.toString()).toBe("https://arena.example.com/");
  });

  it("builds recovery redirects with sanitized next parameters", () => {
    const url = buildRecoveryRedirect("https://arena.example.com", "/arena");
    expect(url.toString()).toBe("https://arena.example.com/auth/update-password?next=%2Farena");
  });
});
