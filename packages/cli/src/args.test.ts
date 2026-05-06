import { helpText, parseArgs } from "./args";

describe("parseArgs", () => {
  it("defaults to local transport when no base URL is provided", () => {
    const options = parseArgs(["play"], {});

    expect(options.transport).toBe("local");
    expect(options.baseUrl).toBe("");
    expect(options.toolsUrl).toBeUndefined();
    expect(options.recordTraining).toBe(true);
    expect(options.sessionMode).toBe("interactive");
    expect(options.sessionId).toBe("cli-play");
    expect(options.agentMaxSteps).toBeUndefined();
    expect(options.sessionLogEnabled).toBe(true);
    expect(options.sessionLogDir).toBe("/tmp");
  });

  it("keeps the default play session stable across repeated runs", () => {
    expect(parseArgs(["play"], {}).sessionId).toBe(parseArgs(["play"], {}).sessionId);
  });

  it("switches to HTTP transport when a base URL is provided", () => {
    const options = parseArgs(["mcp", "--base-url", "http://localhost:3000"]);

    expect(options.transport).toBe("http");
    expect(options.toolsUrl).toBe("http://localhost:3000/api/mcp/tools");
  });

  it("documents the game boy controller restriction for play mode", () => {
    expect(helpText()).toContain("Play mode only sends Game Boy-faithful controller inputs");
  });

  it("documents infinite linked-agent max steps by default", () => {
    expect(helpText()).toContain("--agent-max-steps N     Optional maximum supervised gameplay batches. Default: infinite.");
  });

  it("documents session logging command options", () => {
    expect(helpText()).toContain("--log-dir PATH");
    expect(helpText()).toContain("--log-file PATH");
    expect(helpText()).toContain("--no-session-log");
  });

  it("lets play mode opt out of default training capture", () => {
    const options = parseArgs(["play", "--no-record-training"], {});

    expect(options.recordTraining).toBe(false);
  });

  it("lets sessions choose or disable the default log destination", () => {
    expect(parseArgs(["play", "--log-dir", "/tmp/pokecrystal-debug"], {}).sessionLogDir)
      .toBe("/tmp/pokecrystal-debug");
    expect(parseArgs(["play", "--log-file", "/tmp/session.jsonl"], {}).sessionLogFile)
      .toBe("/tmp/session.jsonl");
    expect(parseArgs(["play", "--no-session-log"], {}).sessionLogEnabled).toBe(false);
  });

  it("keeps explicit play session IDs for deliberate resumes", () => {
    const options = parseArgs(["play", "--session-id", "chris"], {});

    expect(options.sessionId).toBe("chris");
    expect(options.sessionMode).toBe("interactive");
  });

  it("gives play-recorded a repo-local training directory by default", () => {
    const options = parseArgs(["play-recorded", "--session-id", "chris"], {});

    expect(options.recordTraining).toBe(true);
    expect(options.trainingDir).toBe(
      `${process.cwd()}/packages/cli/.tmp-human-play/chris`
    );
  });
});
