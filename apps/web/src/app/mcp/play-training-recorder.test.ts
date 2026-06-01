import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { recordWebTrainingTurn, shouldRecordWebTraining } from "./play-training-recorder";

describe("web play training recorder", () => {
  const originalEnv = process.env;

  beforeEach(() => {
    jest.resetModules();
    process.env = { ...originalEnv };
  });

  afterEach(() => {
    process.env = originalEnv;
  });

  const recordTurn = (sessionId = "dev-session") =>
    recordWebTrainingTurn({
      sessionId,
      baseUrl: "http://localhost:3000",
      rawKey: "ArrowUp",
      action: { type: "move", direction: "up" },
      beforeSnapshot: "OVERWORLD\nBefore",
      actionResultSnapshot: "ok: 1\nch: 1\nfx: moved\nrsn: moved",
      afterSnapshot: "OVERWORLD\nAfter",
      statusSnapshot: "m: overworld\nbat: 0",
      recentEventsSnapshot: "last: moved",
      responseMeta: {
        action_result: [{ ok: 1 }],
        observe: [{ mode: "overworld" }],
        status: [{ mode: "overworld" }],
        recent_events: [{ recap: "moved" }],
      },
    });

  it("records dev-session turns in the agent-compatible training directory layout", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "web-training-"));
    process.env.NODE_ENV = "development";
    process.env.POKECRYSTAL_WEB_TRAINING_DIR = tmpDir;

    recordTurn();

    const trainingDir = path.join(tmpDir, "dev-session");
    const manifest = JSON.parse(fs.readFileSync(path.join(trainingDir, "manifest.json"), "utf8"));
    const episodeRows = fs
      .readFileSync(path.join(trainingDir, "episode.jsonl"), "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    const eventRows = fs
      .readFileSync(path.join(trainingDir, "events.jsonl"), "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));

    expect(manifest.session_id).toBe("dev-session");
    expect(manifest.thread_id).toBe("dev-session");
    expect(manifest.model).toBe("web-dev-gameplay");
    expect(manifest.total_turns).toBe(1);
    expect(manifest.chat_finetune_path).toContain("chat-finetune.jsonl");
    expect(episodeRows[0]).toMatchObject({
      session_id: "dev-session",
      thread_id: "dev-session",
      model: "web-dev-gameplay",
      observer_text: "OVERWORLD\nBefore",
      action_result: "ok: 1\nch: 1\nfx: moved\nrsn: moved",
      decision: {
        actionType: "move",
        direction: "up",
      },
      tags: {
        changed: true,
        noProgress: false,
        toolError: false,
      },
    });
    expect(eventRows).toHaveLength(2);
    expect(eventRows[0]?.type).toBe("decision_completed");
    expect(eventRows[1]?.type).toBe("action_completed");
  });

  it("archives oversized JSONL files before appending more training", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "web-training-"));
    process.env.NODE_ENV = "development";
    process.env.POKECRYSTAL_WEB_TRAINING_DIR = tmpDir;
    process.env.POKECRYSTAL_WEB_MAX_TRAINING_JSONL_BYTES = "10";

    recordTurn("rotation-session");
    recordTurn("rotation-session");

    const trainingDir = path.join(tmpDir, "rotation-session");
    const files = fs.readdirSync(trainingDir);
    const manifest = JSON.parse(fs.readFileSync(path.join(trainingDir, "manifest.json"), "utf8"));
    const activeEpisodeRows = fs.readFileSync(path.join(trainingDir, "episode.jsonl"), "utf8").trim().split("\n");

    expect(files.some((name) => name.startsWith("episode.jsonl.oversized-"))).toBe(true);
    expect(files.some((name) => name.startsWith("events.jsonl.oversized-"))).toBe(true);
    expect(manifest.total_turns).toBe(1);
    expect(activeEpisodeRows).toHaveLength(1);
  });

  it("defaults to recording in development and can be disabled explicitly", () => {
    process.env.NODE_ENV = "development";
    delete process.env.POKECRYSTAL_WEB_RECORD_TRAINING;
    expect(shouldRecordWebTraining()).toBe(true);

    process.env.POKECRYSTAL_WEB_RECORD_TRAINING = "0";
    expect(shouldRecordWebTraining()).toBe(false);
  });
});
