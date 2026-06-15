/** @jest-environment jsdom */
import React, { act } from "react";
import { createRoot } from "react-dom/client";
import { DesktopMcpPanel } from "./desktop-mcp-panel";
import { PRIMARY_MCP_SESSION_ID } from "@/app/mcp/session-id";

describe("DesktopMcpPanel", () => {
  beforeAll(() => {
    (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  });

  afterAll(() => {
    delete (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT;
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("shows the existing streamable HTTP endpoint without bootstrapping another server", async () => {
    const writeText = jest.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    window.history.replaceState({}, "", "/desktop");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<DesktopMcpPanel />);
      await Promise.resolve();
    });

    const endpointPath = `/api/mcp?session_id=${encodeURIComponent(PRIMARY_MCP_SESSION_ID)}`;
    expect(container.textContent).toContain("Existing MCP Server");
    expect(container.textContent).toContain("Streamable HTTP Endpoint");
    expect(container.textContent).toContain(endpointPath);
    expect(container.textContent).toContain("No separate server is created here.");
    expect(container.textContent).toContain('"transport": "streamable-http"');
    expect(container.textContent).not.toContain("Direct MCP Input");

    const copyButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Copy URL"
    );
    expect(copyButton).toBeTruthy();

    await act(async () => {
      copyButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });

    expect(writeText).toHaveBeenCalledWith(expect.stringContaining(endpointPath));

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });
});
