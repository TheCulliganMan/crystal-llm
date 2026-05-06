"use client";

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { DebugLogEntry, getDebugLogEntries, subscribeDebugLog } from "@pokecrystal/core/core/debug-log";

const MAX_LOG_LINES = 500;

const formatTime = (timestamp: number): string => {
  const date = new Date(timestamp);
  const hh = String(date.getHours()).padStart(2, "0");
  const mm = String(date.getMinutes()).padStart(2, "0");
  const ss = String(date.getSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
};

export const DebugLogPanel = React.memo(() => {
  const [entries, setEntries] = useState<DebugLogEntry[]>(() => getDebugLogEntries());
  const logContainerRef = useRef<HTMLPreElement | null>(null);
  const [copyStatus, setCopyStatus] = useState<"idle" | "copied" | "error">("idle");

  useEffect(() => subscribeDebugLog(setEntries), []);

  const lines = useMemo(() => {
    return entries.slice(-MAX_LOG_LINES).map((entry) => {
      const time = formatTime(entry.timestamp);
      return `${time} ${entry.message}`;
    });
  }, [entries]);

  useEffect(() => {
    const container = logContainerRef.current;
    if (!container) {
      return;
    }
    container.scrollTop = container.scrollHeight;
  }, [lines]);

  const handleCopy = useCallback(async () => {
    if (!lines.length) {
      return;
    }
    const payload = lines.join("\n");
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(payload);
      } else {
        const textarea = document.createElement("textarea");
        textarea.value = payload;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
      }
      setCopyStatus("copied");
      window.setTimeout(() => setCopyStatus("idle"), 1500);
    } catch {
      setCopyStatus("error");
      window.setTimeout(() => setCopyStatus("idle"), 2000);
    }
  }, [lines]);

  return (
    <div className="card bg-base-200">
      <div className="card-body gap-2.5">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <p className="text-[0.65rem] uppercase tracking-[0.2em] text-base-content/70">Event Log</p>
            <h3 className="text-lg font-semibold">Runtime activity</h3>
          </div>
          <button
            type="button"
            className="btn btn-sm btn-outline"
            onClick={handleCopy}
            disabled={!lines.length}
          >
            {copyStatus === "copied" ? "Copied" : copyStatus === "error" ? "Copy failed" : "Copy"}
          </button>
        </div>

        <pre
          ref={logContainerRef}
          className="mockup-code m-0 min-h-[8rem] max-h-96 overflow-y-auto whitespace-pre-wrap bg-base-100 text-xs text-base-content/70"
        >
          {lines.length ? lines.join("\n") : "No events yet."}
        </pre>

        <p className="text-xs text-base-content/70">
          Showing the last {Math.min(entries.length, MAX_LOG_LINES)} events.
        </p>
      </div>
    </div>
  );
});

DebugLogPanel.displayName = "DebugLogPanel";

export default DebugLogPanel;
