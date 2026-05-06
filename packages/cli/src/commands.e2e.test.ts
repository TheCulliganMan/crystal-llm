import fs from "node:fs";
import { PassThrough } from "node:stream";
import { runCliCommand, startFakeCliBackend } from "./e2e-helpers";
import { runTextUi } from "./tui";
import type { InkRuntime, TuiViewState } from "./tui-ink";

class FakeTtyInput extends PassThrough {
  public isTTY = true;
  private rawMode = false;

  setRawMode(mode: boolean): void {
    this.rawMode = mode;
  }

  getRawMode(): boolean {
    return this.rawMode;
  }
}

class FakeTtyOutput extends PassThrough {
  public isTTY = true;
  private collected = "";

  constructor() {
    super();
    this.on("data", (chunk: Buffer | string) => {
      this.collected += Buffer.isBuffer(chunk) ? chunk.toString("utf8") : chunk;
    });
  }

  readText(): string {
    return this.collected;
  }
}

const isTuiViewState = (state: unknown): state is TuiViewState =>
  Boolean(
    state &&
      typeof state === "object" &&
      "title" in state &&
      "snapshot" in state &&
      (state as Partial<TuiViewState>).snapshot
  );

const writeTuiState = (stdout: FakeTtyOutput, state: TuiViewState): void => {
  stdout.write([state.title, state.snapshot.statusLine, ...state.snapshot.viewport, ...state.snapshot.menu, ...state.snapshot.prompt, ...state.snapshot.dialogue, ...state.snapshot.actions].join("\n"));
};

const createCommandTestInkRuntime = (stdout: FakeTtyOutput): InkRuntime => ({
  React: {
    createElement: (type: unknown, props?: Record<string, unknown> | null, ...children: unknown[]) => {
      if (typeof type === "function") {
        return (type as (props: Record<string, unknown>) => unknown)({ ...(props ?? {}), children });
      }
      return { type, props, children };
    },
    useEffect: (effect) => {
      effect();
    },
    useState: <T,>(initial: T | (() => T)): [T, (next: T | ((previous: T) => T)) => void] => {
      let value = typeof initial === "function" ? (initial as () => T)() : initial;
      if (isTuiViewState(value)) {
        writeTuiState(stdout, value);
      }
      return [
        value,
        (next) => {
          value = typeof next === "function" ? (next as (previous: T) => T)(value) : next;
          if (isTuiViewState(value)) {
            writeTuiState(stdout, value);
          }
        },
      ];
    },
  },
  ink: {
    Box: "Box",
    Text: "Text",
    render: () => ({ unmount: () => undefined }),
  },
});

describe("pokecrystal-cli command e2e", () => {
  jest.setTimeout(20_000);

  it("prints the packaged skill path and skill contents", async () => {
    const skillPath = await runCliCommand(["skill"]);
    const printedSkill = await runCliCommand(["skill", "--print"]);

    expect(skillPath.stdout).toContain("/packages/cli/skills/pokecrystal-cli/SKILL.md");
    expect(printedSkill.stdout).toContain("PokeCrystal CLI Skill");
  });

  it("register command bootstraps identity and session secret end to end", async () => {
    const backend = await startFakeCliBackend();
    try {
      const result = await runCliCommand([
        "register",
        "--base-url",
        backend.baseUrl,
        "--session-id",
        "register-session",
        "--agent-id",
        "oak-runner",
        "--identity-name",
        "trainer-oak",
      ]);

      expect(result.stdout).toContain("\"token\":\"token-123\"");
      expect(result.stdout).toContain("\"sessionSecret\": \"secret-123\"");

      const registerRequest = backend.requests.find((request) => request.method === "POST");
      const secretRequest = backend.requests.find((request) => request.url.startsWith("/api/arena/session-secret"));
      expect((registerRequest?.body as any)?.tool).toBe("register_identity");
      expect(secretRequest?.headers.authorization).toBe("Bearer token-123");
    } finally {
      await backend.close();
    }
  });

  it("play command runs the real text UI loop and sends gameplay inputs end to end", async () => {
    const backend = await startFakeCliBackend();
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "play-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("w");
      stdin.write(".");
      await new Promise((resolve) => setTimeout(resolve, 300));
      stdin.write(":q!\r");
      await playPromise;

      const rendered = stdout.readText();
      expect(rendered).toContain("PokeCrystal CLI");
      expect(rendered).toContain("TEST OBSERVE");
      expect(
        backend.requests.some(
          (request) => (request.body as any)?.tool === "move"
        )
      ).toBe(true);
      expect(
        backend.requests.filter((request) => (request.body as any)?.tool === "status").length
      ).toBeGreaterThanOrEqual(2);
      expect(stdin.getRawMode()).toBe(false);
    } finally {
      await backend.close();
    }
  });

  it("play command consumes buffered key chunks as sequential controller inputs", async () => {
    const backend = await startFakeCliBackend();
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "buffered-input-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("aa.");
      await new Promise((resolve) => setTimeout(resolve, 300));
      stdin.write(":q!\r");
      await playPromise;

      const moveRequests = backend.requests.filter(
        (request) =>
          (request.body as any)?.tool === "move" &&
          (request.body as any)?.input?.direction === "left"
      );
      expect(moveRequests).toHaveLength(2);
      expect(stdout.readText()).toContain("PokeCrystal CLI");
    } finally {
      await backend.close();
    }
  });

  it("auto-recovers from temporary busy battle states by stepping frames", async () => {
    const backend = await startFakeCliBackend({ scenario: "battle-stall" });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "battle-stall-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("j");
      await new Promise((resolve) => setTimeout(resolve, 300));
      stdin.write(":q!\r");
      await playPromise;

      const observeCalls = backend.requests.filter(
        (request) => (request.body as any)?.tool === "observe"
      );
      expect(observeCalls.length).toBeGreaterThanOrEqual(2);
      expect(stdout.readText()).toContain("PokeCrystal CLI");
    } finally {
      await backend.close();
    }
  });

  it("recovers from a full-length busy battle transition instead of leaving the CLI looking frozen", async () => {
    const backend = await startFakeCliBackend({ scenario: "battle-long-stall" });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "battle-long-stall-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("j");
      await new Promise((resolve) => setTimeout(resolve, 400));
      stdin.write(":q!\r");
      await playPromise;

      const observeCalls = backend.requests.filter(
        (request) => (request.body as any)?.tool === "observe"
      );
      expect(observeCalls.length).toBeGreaterThanOrEqual(10);
      expect(stdout.readText()).toContain("Resolved.");
    } finally {
      await backend.close();
    }
  });

  it("lets a wait input carry a long battle transition through to the live battle screen", async () => {
    const backend = await startFakeCliBackend({ scenario: "battle-transition-wait" });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "battle-transition-wait-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write(".");
      await new Promise((resolve) => setTimeout(resolve, 400));
      stdin.write(":q!\r");
      await playPromise;

      const observeCalls = backend.requests.filter(
        (request) => (request.body as any)?.tool === "observe"
      );
      expect(observeCalls.length).toBeGreaterThanOrEqual(10);
      expect(stdout.readText()).toContain("Resolved.");
    } finally {
      await backend.close();
    }
  });

  it("settles a late battle prompt after move confirm instead of leaving a blank battle frame", async () => {
    const backend = await startFakeCliBackend({ scenario: "battle-late-prompt" });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "battle-late-prompt-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("j");
      await new Promise((resolve) => setTimeout(resolve, 500));
      stdin.write(":q!\r");
      await playPromise;

      expect(stdout.readText()).toContain("CYNDAQUIL used");
    } finally {
      await backend.close();
    }
  });

  it("skips battle animation and sound wait snapshots even when the action result is not busy", async () => {
    const backend = await startFakeCliBackend({ scenario: "battle-observe-delay" });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "battle-observe-delay-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("j");
      await new Promise((resolve) => setTimeout(resolve, 500));
      stdin.write(":q!\r");
      await playPromise;

      const observeAdvanceCalls = backend.requests.filter(
        (request) =>
          (request.body as any)?.tool === "observe" &&
          Number((request.body as any)?.input?.advance_frames ?? 0) > 0
      );
      expect(observeAdvanceCalls.length).toBeGreaterThanOrEqual(2);
      expect(stdout.readText()).toContain("CYNDAQUIL used");
      expect(stdout.readText()).not.toContain("Wait: move animation sound delay");
    } finally {
      await backend.close();
    }
  });

  const runInstantBattleMenuNavigation = async (scenario: "instant-wild-menu" | "instant-trainer-menu") => {
    const backend = await startFakeCliBackend({ scenario });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: `${scenario}-session`,
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      for (const key of ["s", "j", "x", "w", "j"]) {
        stdin.write(key);
        await new Promise((resolve) => setTimeout(resolve, 20));
      }
      await new Promise((resolve) => setTimeout(resolve, 250));
      stdin.write(":q!\r");
      await playPromise;

      const toolCallSignature = backend.requests
        .map((request) => {
          const body = request.body as any;
          return {
            tool: body?.tool,
            input: body?.input,
          };
        })
        .filter((request) => request.tool);
      const actionCalls = backend.requests.filter((request) => {
        const tool = (request.body as any)?.tool;
        return tool === "move" || tool === "press";
      });
      const recoveryAdvanceCalls = backend.requests.filter(
        (request) =>
          (request.body as any)?.tool === "observe" &&
          Number((request.body as any)?.input?.advance_frames ?? 0) > 0
      );
      const observeCalls = backend.requests.filter((request) => (request.body as any)?.tool === "observe");
      const statusCalls = backend.requests.filter((request) => (request.body as any)?.tool === "status");
      const recentEventsCalls = backend.requests.filter((request) => (request.body as any)?.tool === "recent_events");

      expect(actionCalls).toHaveLength(5);
      expect(recoveryAdvanceCalls).toHaveLength(0);
      expect(observeCalls.length).toBeLessThanOrEqual(1);
      expect(statusCalls.length).toBeLessThanOrEqual(1);
      expect(recentEventsCalls.length).toBeLessThanOrEqual(1);
      expect(stdout.readText()).toContain("PokeCrystal CLI");
      return {
        toolCallSignature,
        actionInputs: actionCalls.map((request) => (request.body as any)?.input),
        recoveryAdvanceCalls: recoveryAdvanceCalls.length,
        observeCalls: observeCalls.length,
        statusCalls: statusCalls.length,
        recentEventsCalls: recentEventsCalls.length,
      };
    } finally {
      await backend.close();
    }
  };

  it("keeps instant trainer battle TUI menu input identical to wild battle input", async () => {
    const wild = await runInstantBattleMenuNavigation("instant-wild-menu");
    const trainer = await runInstantBattleMenuNavigation("instant-trainer-menu");

    expect(trainer.toolCallSignature).toEqual(wild.toolCallSignature);
    expect(trainer.actionInputs).toEqual(wild.actionInputs);
    expect(trainer.recoveryAdvanceCalls).toBe(0);
    expect(trainer.observeCalls).toBe(wild.observeCalls);
    expect(trainer.statusCalls).toBe(wild.statusCalls);
    expect(trainer.recentEventsCalls).toBe(wild.recentEventsCalls);
  });

  it("play command can progress through a starter pickup and first battle timing loop", async () => {
    const backend = await startFakeCliBackend({ scenario: "first-battle" });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "first-battle-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("a"); // walk toward Elm's Lab
      stdin.write("a");
      stdin.write("w");
      stdin.write("w");
      stdin.write("j"); // pick starter
      stdin.write("s");
      stdin.write("s");
      stdin.write("s"); // walk into tall grass
      stdin.write("j");
      stdin.write("j");
      stdin.write("j"); // finish first battle turns
      await new Promise((resolve) => setTimeout(resolve, 250));
      stdin.write(":q!\r");
      await playPromise;

      const rendered = stdout.readText();
      expect(rendered).toContain("PokeCrystal CLI");
      expect(rendered).toContain("ROUTE 29");
      expect(
        backend.requests.filter(
          (request) => (request.body as any)?.tool === "press"
        ).length
      ).toBeGreaterThanOrEqual(4);
      expect(
        backend.requests.filter((request) => (request.body as any)?.tool === "status").length
      ).toBeGreaterThanOrEqual(2);
      expect(stdin.getRawMode()).toBe(false);
    } finally {
      await backend.close();
    }
  });

  it("stops processing buffered movement keys once a wild battle starts", async () => {
    const backend = await startFakeCliBackend({ scenario: "first-battle" });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "buffered-battle-handoff-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("aawwssssssssssss");
      await new Promise((resolve) => setTimeout(resolve, 200));
      stdin.write(":q!\r");
      await playPromise;

      const downwardMoves = backend.requests.filter(
        (request) =>
          (request.body as any)?.tool === "move" &&
          (request.body as any)?.input?.direction === "down"
      );
      expect(downwardMoves).toHaveLength(8);
      expect(stdout.readText()).toContain("A wild Pidgey appeared!");
    } finally {
      await backend.close();
    }
  });

  it("stops processing buffered A presses after the first in-battle confirm so prompts do not get overrun", async () => {
    const backend = await startFakeCliBackend({ scenario: "first-battle" });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "buffered-battle-a-stop-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("aawwssssssssssss");
      await new Promise((resolve) => setTimeout(resolve, 200));
      stdin.write("jjj");
      await new Promise((resolve) => setTimeout(resolve, 200));
      stdin.write(":q!\r");
      await playPromise;

      const pressRequests = backend.requests.filter(
        (request) => (request.body as any)?.tool === "press"
      );
      expect(pressRequests).toHaveLength(1);
      expect(stdout.readText()).toContain("A wild Pidgey appeared!");
    } finally {
      await backend.close();
    }
  });

  it("stops processing buffered A presses after an NPC dialogue closes so it does not immediately reopen", async () => {
    const backend = await startFakeCliBackend({ scenario: "npc-dialogue-close" });
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    try {
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "buffered-dialogue-a-stop-session",
          recordTraining: false,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("jjj");
      await new Promise((resolve) => setTimeout(resolve, 200));
      stdin.write(":q!\r");
      await playPromise;

      const pressRequests = backend.requests.filter(
        (request) => (request.body as any)?.tool === "press"
      );
      expect(pressRequests).toHaveLength(2);
      expect(stdout.readText()).toContain("Talk to the berry man.");
    } finally {
      await backend.close();
    }
  });

  it("play mode records training data during live play when enabled", async () => {
    const backend = await startFakeCliBackend();
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const trainingDir = `${process.cwd()}/.tmp-cli-play-training`;
    try {
      await fs.promises.rm(trainingDir, { recursive: true, force: true });
      const playPromise = runTextUi(
        {
          command: "play",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId: "recorded-play-session",
          recordTraining: true,
          trainingDir,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("w");
      await new Promise((resolve) => setTimeout(resolve, 100));
      stdin.write(":q!\r");
      await playPromise;

      const manifest = JSON.parse(await fs.promises.readFile(`${trainingDir}/manifest.json`, "utf8"));
      const episode = (await fs.promises.readFile(`${trainingDir}/episode.jsonl`, "utf8"))
        .trim()
        .split("\n")
        .map((line) => JSON.parse(line));

      expect(manifest.total_turns).toBe(1);
      expect(episode[0]).toMatchObject({
        session_id: "recorded-play-session",
        raw_key: "w",
        tool_name: "move",
        tool_input: {
          direction: "up",
          steps: 1,
        },
        after_snapshot: "TEST OBSERVE\nX=7 Y=3",
        status_snapshot: "{\"mode\":\"overworld\",\"map\":\"TEST MAP\",\"coords\":{\"x\":7,\"y\":3},\"can_move\":true}",
        tags: expect.arrayContaining(["move"]),
      });
      expect(episode[0].before_snapshot).toContain("TEST OBSERVE");
      expect(episode[0].recent_events_snapshot).toContain("n: 1");
    } finally {
      await backend.close();
      await fs.promises.rm(trainingDir, { recursive: true, force: true });
    }
  });

  it("play-recorded uses a repo-local training directory by default so a human can resume recorded runs easily", async () => {
    const backend = await startFakeCliBackend();
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const sessionId = "human-recorded-session";
    const trainingDir = `${process.cwd()}/packages/cli/.tmp-human-play/${sessionId}`;
    try {
      await fs.promises.rm(trainingDir, { recursive: true, force: true });
      const playPromise = runTextUi(
        {
          command: "play-recorded",
          transport: "http",
          baseUrl: backend.baseUrl,
          toolsUrl: `${backend.baseUrl}/api/mcp/tools`,
          sessionId,
          recordTraining: true,
          trainingDir,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & {
            isTTY?: boolean;
            setRawMode?(mode: boolean): void;
          },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createCommandTestInkRuntime(stdout),
        }
      );

      await new Promise((resolve) => setTimeout(resolve, 50));
      stdin.write("w");
      await new Promise((resolve) => setTimeout(resolve, 100));
      stdin.write(":q!\r");
      await playPromise;

      const manifest = JSON.parse(await fs.promises.readFile(`${trainingDir}/manifest.json`, "utf8"));
      expect(manifest.training_dir).toBe(trainingDir);
      expect(manifest.session_id).toBe(sessionId);
      expect(stdout.readText()).toContain("PokeCrystal CLI");
    } finally {
      await backend.close();
      await fs.promises.rm(trainingDir, { recursive: true, force: true });
    }
  });
});
