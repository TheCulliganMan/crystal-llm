import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  createSessionLogger,
  resolveSessionLogFile,
  summarizeToolResult,
  withResolvedSessionLogFile,
} from "./session-log";
import type { CliOptions } from "./types";

const createOptions = (logDir: string): CliOptions => ({
  command: "play",
  transport: "local",
  baseUrl: "",
  sessionId: "battle/session",
  sessionLogEnabled: true,
  sessionLogDir: logDir,
});

describe("session logging", () => {
  it("resolves play logs into the configured directory with a safe session name", () => {
    const file = resolveSessionLogFile(createOptions("/tmp/pokecrystal-logs"), Date.UTC(2026, 4, 3));

    expect(file).toContain("/tmp/pokecrystal-logs/");
    expect(path.basename(file ?? "")).toBe("pokecrystal-battle_session.jsonl");
  });

  it("resolves default session log files when logging is enabled", () => {
    const options = withResolvedSessionLogFile(createOptions("/tmp/pokecrystal-logs"), Date.UTC(2026, 4, 3));

    expect(options.sessionLogFile).toContain("/tmp/pokecrystal-logs/");
  });

  it("reuses one rolling log file per session id", () => {
    const logDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-session-log-"));
    const first = withResolvedSessionLogFile(createOptions(logDir), Date.UTC(2026, 4, 3));
    const second = withResolvedSessionLogFile(createOptions(logDir), Date.UTC(2026, 4, 4));

    expect(first.sessionLogFile).toBe(second.sessionLogFile);
  });

  it("writes JSONL entries and redacts sensitive payload fields", () => {
    const logDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-session-log-"));
    const logFile = path.join(logDir, "session.jsonl");
    const logger = createSessionLogger({
      sessionId: "session-1",
      sessionLogEnabled: true,
      sessionLogFile: logFile,
    });

    logger.write("tool_call", {
      token: "token-123",
      input: {
        sessionSecret: "secret-123",
        visible: "ok",
      },
    });

    const [entry] = fs.readFileSync(logFile, "utf8").trim().split("\n").map((line) => JSON.parse(line));
    expect(entry).toMatchObject({
      session_id: "session-1",
      event: "tool_call",
      token: "[redacted]",
      input: {
        sessionSecret: "[redacted]",
        visible: "ok",
      },
    });
  });

  it("keeps only the latest 10000 JSONL entries", () => {
    const logDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-session-log-"));
    const logFile = path.join(logDir, "session.jsonl");
    fs.writeFileSync(
      logFile,
      `${Array.from({ length: 10_000 }, (_value, index) => JSON.stringify({ index })).join("\n")}\n`,
      "utf8",
    );
    const logger = createSessionLogger({
      sessionId: "session-1",
      sessionLogEnabled: true,
      sessionLogFile: logFile,
    });

    for (let index = 0; index < 501; index += 1) {
      logger.write("latest", { index });
    }

    const entries = fs.readFileSync(logFile, "utf8").trim().split("\n").map((line) => JSON.parse(line));
    expect(entries).toHaveLength(10_000);
    expect(entries[0]).toEqual({ index: 501 });
    expect(entries.at(-1)).toMatchObject({ event: "latest", index: 500 });
  });

  it("does not reread and trim the log file on every append", () => {
    const logDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-session-log-"));
    const logFile = path.join(logDir, "session.jsonl");
    fs.writeFileSync(logFile, `${JSON.stringify({ index: 0 })}\n`, "utf8");
    const readSpy = jest.spyOn(fs, "readFileSync");
    const logger = createSessionLogger({
      sessionId: "session-1",
      sessionLogEnabled: true,
      sessionLogFile: logFile,
    });

    logger.write("one");
    logger.write("two");

    expect(readSpy.mock.calls.filter((call) => call[0] === logFile)).toHaveLength(1);
    readSpy.mockRestore();
  });

  it("summarizes binary tool results without logging image data", () => {
    const summary = summarizeToolResult({
      content: [
        { type: "text", text: "status" },
        { type: "image", data: "abc123", mimeType: "image/png" },
      ],
    });

    expect(summary["content"]).toEqual([
      { type: "text", text: "status", text_length: 6 },
      { type: "image", mime_type: "image/png", data_length: 6 },
    ]);
  });
});
