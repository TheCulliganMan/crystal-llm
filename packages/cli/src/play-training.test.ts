import fs from "node:fs";
import path from "node:path";
import { createPlayTrainingRecorder } from "./play-training";
import type { CliOptions, ToolResult } from "./types";

const createOptions = (trainingDir: string): CliOptions => ({
  command: "play",
  transport: "local",
  baseUrl: "",
  sessionId: "training-session",
  recordTraining: true,
  trainingDir,
});

const textResult = (text: string): ToolResult => ({
  content: [{ type: "text", text }],
});

describe("createPlayTrainingRecorder", () => {
  it("archives legacy schema files before writing new rows", () => {
    const trainingDir = path.join(process.cwd(), "packages/cli/.tmp-play-training-legacy");
    fs.rmSync(trainingDir, { recursive: true, force: true });
    fs.mkdirSync(trainingDir, { recursive: true });
    fs.writeFileSync(
      path.join(trainingDir, "manifest.json"),
      `${JSON.stringify({
        session_id: "training-session",
        created_at: "2026-03-28T00:00:00.000Z",
        updated_at: "2026-03-28T00:00:00.000Z",
        transport: "local",
        base_url: "",
        training_dir: trainingDir,
        episode_path: path.join(trainingDir, "episode.jsonl"),
        total_turns: 1,
      })}\n`
    );
    fs.writeFileSync(
      path.join(trainingDir, "episode.jsonl"),
      `${JSON.stringify({
        session_id: "training-session",
        recorded_at: "2026-03-28T00:00:00.000Z",
        step_index: 1,
        raw_key: "d",
        action: { type: "move", direction: "right" },
        tool_name: "move",
        tool_input: { direction: "right", steps: 1 },
        before_snapshot: "old",
        after_snapshot: "old",
        response_meta: [],
        transport: "local",
      })}\n`
    );

    const recorder = createPlayTrainingRecorder(createOptions(trainingDir));
    expect(recorder).not.toBeNull();

    const archivedFiles = fs.readdirSync(trainingDir).filter((name) => name.includes(".legacy-"));
    expect(archivedFiles.some((name) => name.startsWith("episode.jsonl.legacy-"))).toBe(true);
    expect(archivedFiles.some((name) => name.startsWith("manifest.json.legacy-"))).toBe(true);

    const manifest = JSON.parse(fs.readFileSync(path.join(trainingDir, "manifest.json"), "utf8"));
    expect(manifest.schema_version).toBe(2);
    expect(manifest.total_turns).toBe(0);
    expect(manifest.skipped_turns).toBe(0);
    expect(manifest.example_turns).toBe(0);
    expect(manifest.examples_path).toContain("examples.jsonl");
  });

  it("suppresses repeated no-progress turns for the same state and writes curated examples", () => {
    const trainingDir = path.join(process.cwd(), "packages/cli/.tmp-play-training-dedupe");
    fs.rmSync(trainingDir, { recursive: true, force: true });
    const recorder = createPlayTrainingRecorder(createOptions(trainingDir));
    expect(recorder).not.toBeNull();

    const before = textResult("OVERWORLD\nPos: (29,31)");
    const actionResult = textResult(
      [
        "ctx:",
        "a:",
        "  ok: 0",
        "  ch: 0",
        "  fx: blocked",
        "  rsn: blocked",
      ].join("\n")
    );
    const observed = textResult("OVERWORLD\nPos: (29,31)");
    const status = textResult("m: overworld\nmap: Route29\nxy[2\t]: 29\t31");
    const recent = textResult('sum: "move:left:1 blocked @ Route29 29,31"\nn: 10');

    for (let index = 0; index < 6; index += 1) {
      recorder!.recordTurn({
        rawKey: index % 2 === 0 ? "a" : "w",
        action: index % 2 === 0 ? { type: "move", direction: "left" } : { type: "move", direction: "up" },
        beforeResult: before,
        actionResult,
        observedAfterResult: observed,
        statusResult: status,
        recentEventsResult: recent,
      });
    }

    const rows = fs
      .readFileSync(path.join(trainingDir, "episode.jsonl"), "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    const examplesPath = path.join(trainingDir, "examples.jsonl");
    const manifest = JSON.parse(fs.readFileSync(path.join(trainingDir, "manifest.json"), "utf8"));

    expect(rows).toHaveLength(3);
    expect(manifest.total_turns).toBe(3);
    expect(manifest.skipped_turns).toBe(3);
    expect(manifest.example_turns).toBe(0);
    expect(fs.existsSync(examplesPath)).toBe(false);
    expect(rows[0].tags).toEqual(expect.arrayContaining(["no-progress", "reason:blocked"]));
  });

  it("keeps progress and discovery turns in examples.jsonl", () => {
    const trainingDir = path.join(process.cwd(), "packages/cli/.tmp-play-training-examples");
    fs.rmSync(trainingDir, { recursive: true, force: true });
    const recorder = createPlayTrainingRecorder(createOptions(trainingDir));
    expect(recorder).not.toBeNull();

    recorder!.recordTurn({
      rawKey: ".",
      action: { type: "wait", frames: 8 },
      beforeResult: textResult("OVERWORLD\nPos: (79,33)"),
      actionResult: textResult("OVERWORLD\nPos: (79,33)"),
      observedAfterResult: textResult("BATTLE\n\nDIALOGUE\nWild HOOTHOOT\nappeared!"),
      statusResult: textResult("m: battle\nmap: Route29\nxy[2\t]: 81\t33"),
      recentEventsResult: textResult('sum: "mode:overworld->battle @ Route29 81,33"\nn: 17'),
    });

    recorder!.recordTurn({
      rawKey: "j",
      action: { type: "press", button: "a" },
      beforeResult: textResult("BATTLE\n\nDIALOGUE\nWild HOOTHOOT\nappeared!"),
      actionResult: textResult(["ctx:", "a:", "  ok: 1", "  ch: 1", "  fx: changed"].join("\n")),
      observedAfterResult: textResult("BATTLE\n\nMENU\n▶ TACKLE"),
      statusResult: textResult("m: battle\nmenu: 1\nmap: Route29\nxy[2\t]: 81\t33"),
      recentEventsResult: textResult('sum: "menu opened @ Route29 81,33"\nn: 40'),
    });

    const manifest = JSON.parse(fs.readFileSync(path.join(trainingDir, "manifest.json"), "utf8"));
    const exampleRows = fs
      .readFileSync(path.join(trainingDir, "examples.jsonl"), "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));

    expect(manifest.total_turns).toBe(2);
    expect(manifest.example_turns).toBe(2);
    expect(exampleRows).toHaveLength(2);
    expect(exampleRows[0].after_snapshot).toContain("Wild HOOTHOOT");
    expect(exampleRows[1].tags).toEqual(expect.arrayContaining(["progress"]));
  });

  it("persists agent stream events beside training turns", () => {
    const trainingDir = path.join(process.cwd(), "packages/cli/.tmp-play-training-agent-events");
    fs.rmSync(trainingDir, { recursive: true, force: true });
    const recorder = createPlayTrainingRecorder(createOptions(trainingDir));
    expect(recorder).not.toBeNull();

    recorder!.recordAgentEvent({
      type: "status",
      source: "runner",
      message: "direct player batch produced no real gameplay progress; non-agentic recovery disabled",
    });

    const rows = fs
      .readFileSync(path.join(trainingDir, "agent-events.jsonl"), "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    const manifest = JSON.parse(fs.readFileSync(path.join(trainingDir, "manifest.json"), "utf8"));

    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({
      type: "status",
      source: "runner",
      text: "direct player batch produced no real gameplay progress; non-agentic recovery disabled",
    });
    expect(manifest.agent_events_path).toContain("agent-events.jsonl");
    expect(manifest.total_agent_events).toBe(1);
  });

  it("continues no-progress dedupe after creating a new recorder from existing rows", () => {
    const trainingDir = path.join(process.cwd(), "packages/cli/.tmp-play-training-resume");
    fs.rmSync(trainingDir, { recursive: true, force: true });
    const firstRecorder = createPlayTrainingRecorder(createOptions(trainingDir));
    expect(firstRecorder).not.toBeNull();

    const baseTurn = {
      beforeResult: textResult("OVERWORLD\nPos: (29,31)"),
      actionResult: textResult(["ctx:", "a:", "  ok: 0", "  ch: 0", "  fx: blocked", "  rsn: blocked"].join("\n")),
      observedAfterResult: textResult("OVERWORLD\nPos: (29,31)"),
      statusResult: textResult("m: overworld\nmap: Route29\nxy[2\t]: 29\t31"),
      recentEventsResult: textResult('sum: "move:left blocked @ Route29 29,31"\nn: 10'),
    };
    for (let index = 0; index < 3; index += 1) {
      firstRecorder!.recordTurn({
        rawKey: "a",
        action: { type: "move", direction: "left" },
        ...baseTurn,
      });
    }

    const resumedRecorder = createPlayTrainingRecorder(createOptions(trainingDir));
    expect(resumedRecorder).not.toBeNull();
    resumedRecorder!.recordTurn({
      rawKey: "a",
      action: { type: "move", direction: "left" },
      ...baseTurn,
    });

    const rows = fs
      .readFileSync(path.join(trainingDir, "episode.jsonl"), "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    const manifest = JSON.parse(fs.readFileSync(path.join(trainingDir, "manifest.json"), "utf8"));

    expect(rows).toHaveLength(3);
    expect(manifest.total_turns).toBe(3);
    expect(manifest.skipped_turns).toBe(1);
  });

  it("archives oversized current training logs without reading them into memory", () => {
    const trainingDir = path.join(process.cwd(), "packages/cli/.tmp-play-training-oversized");
    fs.rmSync(trainingDir, { recursive: true, force: true });
    fs.mkdirSync(trainingDir, { recursive: true });
    const episodePath = path.join(trainingDir, "episode.jsonl");
    const examplesPath = path.join(trainingDir, "examples.jsonl");
    const manifestPath = path.join(trainingDir, "manifest.json");
    fs.writeFileSync(
      manifestPath,
      `${JSON.stringify({
        schema_version: 2,
        session_id: "training-session",
        created_at: "2026-05-03T00:00:00.000Z",
        updated_at: "2026-05-03T00:00:00.000Z",
        transport: "local",
        base_url: "",
        training_dir: trainingDir,
        episode_path: episodePath,
        examples_path: examplesPath,
        total_turns: 100,
        skipped_turns: 0,
        example_turns: 100,
        total_agent_events: 0,
      })}\n`
    );
    fs.writeFileSync(episodePath, `${JSON.stringify({
      session_id: "training-session",
      recorded_at: "2026-05-03T00:00:00.000Z",
      step_index: 100,
      raw_key: "a",
      action: { type: "move", direction: "left" },
      tool_name: "move",
      tool_input: { direction: "left", steps: 1 },
      before_snapshot: "OVERWORLD",
      action_result_snapshot: "ok: 1\nch: 1",
      after_snapshot: "OVERWORLD",
      status_snapshot: "m: overworld",
      recent_events_snapshot: "n: 100",
      result_flags: { ok: true, changed: true },
      tags: ["move", "progress"],
      response_meta: [],
      transport: "local",
    })}\n`);
    fs.writeFileSync(examplesPath, "current example row\n");
    fs.truncateSync(episodePath, 129 * 1024 * 1024);
    fs.truncateSync(examplesPath, 129 * 1024 * 1024);
    const readSpy = jest.spyOn(fs, "readFileSync");

    try {
      const recorder = createPlayTrainingRecorder(createOptions(trainingDir));
      expect(recorder).not.toBeNull();
    } finally {
      readSpy.mockRestore();
    }

    expect(readSpy.mock.calls.some((call) => call[0] === episodePath)).toBe(false);
    expect(readSpy.mock.calls.some((call) => call[0] === examplesPath)).toBe(false);
    const archivedFiles = fs.readdirSync(trainingDir).filter((name) => name.includes(".oversized-"));
    expect(archivedFiles.some((name) => name.startsWith("episode.jsonl.oversized-"))).toBe(true);
    expect(archivedFiles.some((name) => name.startsWith("examples.jsonl.oversized-"))).toBe(true);
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    expect(manifest.total_turns).toBe(0);
    expect(manifest.example_turns).toBe(0);
    fs.rmSync(trainingDir, { recursive: true, force: true });
  });
});
