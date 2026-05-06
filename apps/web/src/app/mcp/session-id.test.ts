import { PRIMARY_MCP_SESSION_ID, resolveMcpSessionId } from "./session-id";

describe("resolveMcpSessionId", () => {
  it("pins the primary long-running MCP session id", () => {
    expect(PRIMARY_MCP_SESSION_ID).toBe("ultimate-run");
  });

  it("returns valid provided session ids", () => {
    expect(resolveMcpSessionId("session-123")).toBe("session-123");
  });

  it("falls back to the primary long-running session id when missing", () => {
    expect(resolveMcpSessionId(undefined)).toBe(PRIMARY_MCP_SESSION_ID);
    expect(resolveMcpSessionId("")).toBe(PRIMARY_MCP_SESSION_ID);
  });

  it("falls back when session id is invalid", () => {
    expect(resolveMcpSessionId("bad id")).toBe(PRIMARY_MCP_SESSION_ID);
  });
});
