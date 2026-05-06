/** @jest-environment jsdom */
import React, { act } from "react";
import { createRoot } from "react-dom/client";
import McpPage from "./page";
import { PRIMARY_MCP_SESSION_ID } from "@/app/mcp/session-id";

describe("McpPage session continuity", () => {
  it("pins the displayed endpoint to the primary ultimate-run session", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    const expectedSessionPath = `/api/mcp?session_id=${encodeURIComponent(PRIMARY_MCP_SESSION_ID)}`;
    const expectedToolsPath = `/api/mcp/tools?session_id=${encodeURIComponent(PRIMARY_MCP_SESSION_ID)}`;

    await act(async () => {
      root.render(<McpPage />);
      await Promise.resolve();
    });

    expect(container.querySelector('[data-testid="route-mcp"]')).toBeTruthy();
    const text = container.textContent ?? "";
    expect(text).toContain(`Session id: ${PRIMARY_MCP_SESSION_ID}`);
    expect(text).toContain(expectedSessionPath);
    expect(text).toContain(expectedToolsPath);
    expect(text).toContain("Session Link");
    expect(text).toContain("Direct tools endpoint");
    expect(text).toContain("Fast Start");
    expect(text).toContain("The simplest honest way to play is one bootstrap, one helper, and one small action loop.");
    expect(text).toContain("Play with a simple loop: status, observe if needed, one small action, status again.");
    expect(text).toContain('Authorization: Bearer ${TOKEN}');
    expect(text).toContain('x-session-secret: ${SESSION_SECRET}');
    expect(text).toContain("Download KrabbyClaw API Skill");
    expect(text).toContain("Download KrabbyClawArena Skill");
    expect(text).toContain("Download Progress Tracker Skill");
    expect(text).toContain("map_info: current map, warps, and hotspot metadata");
    expect(text).toContain("flow_state: spoiler-safe progression toward Mt. Silver");
    expect(text).toContain("hold_button: hold a button for N frames");
    expect(text).toContain("optional bounded recovery helper for stuck text flows only");
    expect(text).toContain("Agent API Endpoints");
    expect(text).toContain("/api/mcp/tools?session_id=<agent-id>");
    expect(text).toContain("/api/arena/session-secret?session_id=<agent-id>");
    expect(text).toContain("/api/arena/frame?session_id=<agent-id>&scale=2&advance=0");
    expect(text).toContain("top-level `map` and `flow_state`");
    expect(text).toContain("/api/arena/progress");
    expect(text).toContain("/api/arena/krabbyclaw?limit=16");
    expect(text).toContain("/api/arena/krabbyclaw");
    expect(text).toContain("x-session-secret");

    const downloadLinks = Array.from(container.querySelectorAll('a[download]')).map((link) =>
      link.getAttribute("href")
    );
    expect(downloadLinks).toEqual(
      expect.arrayContaining([
        "/downloads/krabbyclaw-skill.zip",
        "/downloads/krabbyclaw-arena-skill.zip",
        "/downloads/krabbyclaw-progress-tracker-skill.zip",
      ]),
    );

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });
});
