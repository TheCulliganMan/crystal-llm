"use client";

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { PRIMARY_MCP_SESSION_ID } from "@/app/mcp/session-id";

const COPY_RESET_MS = 1200;

const resolveOrigin = (): string => {
  if (typeof window === "undefined") {
    return "";
  }
  return window.location.origin;
};

export const DesktopMcpPanel = React.memo(() => {
  const [origin, setOrigin] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    setOrigin(resolveOrigin());
  }, []);

  useEffect(() => {
    if (!copied) {
      return;
    }
    const timer = window.setTimeout(() => setCopied(false), COPY_RESET_MS);
    return () => window.clearTimeout(timer);
  }, [copied]);

  const streamableHttpUrl = useMemo(
    () => `${origin}/api/mcp?session_id=${encodeURIComponent(PRIMARY_MCP_SESSION_ID)}`,
    [origin]
  );
  const displayUrl = streamableHttpUrl.startsWith("/")
    ? streamableHttpUrl
    : streamableHttpUrl.replace(/^https?:\/\/127\.0\.0\.1(?::\d+)?/, "http://127.0.0.1:<desktop-port>");
  const configSnippet = `{
  "mcpServers": {
    "krabbyclaw": {
      "url": "${displayUrl}",
      "transport": "streamable-http"
    }
  }
}`;

  const handleCopy = useCallback(async () => {
    if (typeof navigator === "undefined" || !navigator.clipboard) {
      return;
    }
    try {
      await navigator.clipboard.writeText(streamableHttpUrl);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }, [streamableHttpUrl]);

  return (
    <section className="space-y-3" data-testid="desktop-mcp-panel">
      <div className="rounded border border-base-300 bg-base-200/60 p-3">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/60">Existing MCP Server</p>
        <h2 className="text-base font-semibold">Streamable HTTP Endpoint</h2>
        <p className="mt-2 text-xs text-base-content/70">
          This uses the app&apos;s existing `/api/mcp` Streamable HTTP transport. No separate server is created here.
        </p>
      </div>

      <div className="space-y-2 rounded border border-base-300 bg-base-100 p-3">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/60">Endpoint</p>
        <code className="block overflow-x-auto rounded bg-base-200 p-2 text-[0.7rem] leading-relaxed">
          {displayUrl}
        </code>
        <div className="grid grid-cols-2 gap-2">
          <button type="button" className="btn btn-sm btn-primary rounded normal-case" onClick={handleCopy}>
            {copied ? "Copied" : "Copy URL"}
          </button>
          <a className="btn btn-sm btn-outline rounded normal-case" href="/mcp" target="_blank" rel="noreferrer">
            Open MCP Page
          </a>
        </div>
      </div>

      <div className="space-y-2 rounded border border-base-300 bg-base-100 p-3">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/60">Client Config</p>
        <pre className="overflow-x-auto rounded bg-base-200 p-2 text-[0.7rem] leading-relaxed">
          <code>{configSnippet}</code>
        </pre>
      </div>
    </section>
  );
});

DesktopMcpPanel.displayName = "DesktopMcpPanel";

export default DesktopMcpPanel;
