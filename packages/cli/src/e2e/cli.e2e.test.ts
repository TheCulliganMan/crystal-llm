import http from "node:http";
import path from "node:path";
import { execFile, execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

type RecordedRequest = {
  method: string;
  path: string;
  body: unknown;
  headers: Record<string, string>;
};

const repoRoot = path.resolve(__dirname, "..", "..", "..", "..");
const cliBinary = path.join(repoRoot, "packages/cli/dist/bin/pokecrystal-cli.js");

const requests: RecordedRequest[] = [];

const readBody = async (request: http.IncomingMessage): Promise<string> => {
  const chunks: Buffer[] = [];
  for await (const chunk of request) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString("utf8");
};

const startFakeBackend = async (): Promise<{ baseUrl: string; close: () => Promise<void> }> => {
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");
    const mayHaveBody =
      request.method !== "GET" &&
      request.method !== "HEAD" &&
      (request.headers["content-length"] !== undefined || request.headers["transfer-encoding"] !== undefined);
    const bodyText = mayHaveBody ? await readBody(request) : "";
    let body: unknown = null;
    if (bodyText) {
      try {
        body = JSON.parse(bodyText);
      } catch {
        body = bodyText;
      }
    }
    requests.push({
      method: request.method ?? "GET",
      path: `${url.pathname}${url.search}`,
      body,
      headers: Object.fromEntries(
        Object.entries(request.headers).map(([key, value]) => [key, Array.isArray(value) ? value.join(",") : value ?? ""])
      ),
    });

    const toolName = (body as { tool?: string } | null)?.tool;

    if (url.pathname === "/api/mcp/tools") {
      response.writeHead(200, { "content-type": "application/json", connection: "close" });
      const result =
        toolName === "register_identity"
          ? {
              ok: true,
              tool: "register_identity",
              result: {
                content: [{ type: "text", text: JSON.stringify({ token: "token-123", playerId: "player-123" }) }],
              },
            }
          : toolName === "observe"
            ? {
                ok: true,
                tool: "observe",
                result: {
                  content: [{ type: "text", text: "HELLO SNAPSHOT" }],
                },
              }
            : toolName === "status"
              ? {
                  ok: true,
                  tool: "status",
                  result: {
                    content: [
                      {
                        type: "text",
                        text: JSON.stringify({
                          status: {
                            mode: "overworld",
                            map: "NEW BARK TOWN",
                            coords: { x: 4, y: 8 },
                            can_move: true,
                          },
                        }),
                      },
                    ],
                  },
                }
              : toolName === "execute_macro"
                ? {
                    ok: true,
                    tool: "execute_macro",
                    result: {
                      content: [{ type: "text", text: "MACRO OK" }],
                    },
                  }
                : toolName === "recent_events"
                  ? {
                      ok: true,
                      tool: "recent_events",
                      result: {
                        content: [{ type: "text", text: JSON.stringify({ recap: "", total: 0, events: [] }) }],
                      },
                    }
                : toolName === "move"
                  ? {
                      ok: true,
                      tool: "move",
                      result: {
                        content: [
                          {
                            type: "text",
                            text: JSON.stringify({
                              moved: true,
                              direction: (body as { input?: { direction?: string } } | null)?.input?.direction,
                            }),
                          },
                        ],
                      },
                    }
                  : toolName === "press"
                    ? {
                        ok: true,
                        tool: "press",
                        result: {
                          content: [{ type: "text", text: "PRESSED" }],
                        },
                      }
                    : toolName === "whoami"
                      ? {
                          ok: true,
                          tool: "whoami",
                          result: {
                            content: [{ type: "text", text: "WHOAMI" }],
                          },
                        }
                      : {
                          ok: true,
                          tool: toolName ?? "unknown",
                          result: {
                            content: [{ type: "text", text: `TOOL:${toolName ?? "unknown"}` }],
                          },
                        };
      response.end(JSON.stringify(result));
      return;
    }

    if (url.pathname === "/api/arena/session-secret") {
      response.writeHead(200, { "content-type": "application/json", connection: "close" });
      response.end(JSON.stringify({ sessionSecret: "secret-abc" }));
      return;
    }

    response.writeHead(404, { "content-type": "text/plain", connection: "close" });
    response.end("not found");
  });

  await new Promise<void>((resolve) => {
    server.listen(0, "127.0.0.1", () => resolve());
  });

  const address = server.address();
  if (!address || typeof address === "string") {
    throw new Error("Failed to start fake backend.");
  }

  return {
    baseUrl: `http://127.0.0.1:${address.port}`,
    close: async () =>
      new Promise<void>((resolve, reject) => {
        server.close((error) => {
          if (error) {
            reject(error);
            return;
          }
          resolve();
        });
      }),
  };
};

const buildCli = (): void => {
  execFileSync("npm", ["run", "build", "--workspace", "@pokecrystal/cli"], {
    cwd: repoRoot,
    stdio: "inherit",
    env: process.env,
  });
};

const waitFor = async (predicate: () => boolean, timeoutMs = 10_000): Promise<void> => {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() > deadline) {
      throw new Error("Timed out waiting for test condition.");
    }
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
};

const runCliBinary = (args: string[]): Promise<{ stdout: string; stderr: string }> =>
  new Promise((resolve, reject) => {
    execFile(
      process.execPath,
      [cliBinary, ...args],
      {
        cwd: repoRoot,
        encoding: "utf8",
        env: process.env,
      },
      (error, stdout, stderr) => {
        if (error) {
          reject(Object.assign(error, { stdout, stderr }));
          return;
        }
        resolve({ stdout, stderr });
      }
    );
  });

type TextToolContent = {
  type: string;
  text?: string;
};

const readFirstTextEntry = (result: unknown): TextToolContent | undefined => {
  if (!result || typeof result !== "object" || !("content" in result)) {
    return undefined;
  }
  const content = (result as { content?: unknown }).content;
  if (!Array.isArray(content)) {
    return undefined;
  }
  const [firstEntry] = content;
  if (!firstEntry || typeof firstEntry !== "object") {
    return undefined;
  }
  const candidate = firstEntry as { type?: unknown; text?: unknown };
  return {
    type: typeof candidate.type === "string" ? candidate.type : "",
    text: typeof candidate.text === "string" ? candidate.text : undefined,
  };
};

describe("pokecrystal-cli end to end", () => {
  jest.setTimeout(120_000);
  let backendBaseUrl = "";
  let closeBackend: (() => Promise<void>) | null = null;

  beforeAll(async () => {
    buildCli();
    const backend = await startFakeBackend();
    backendBaseUrl = backend.baseUrl;
    closeBackend = backend.close;
  });

  afterAll(async () => {
    if (closeBackend) {
      await closeBackend();
    }
  });

  beforeEach(() => {
    requests.length = 0;
  });

  it("serves MCP tools over stdio", async () => {
    const transport = new StdioClientTransport({
      command: process.execPath,
      args: [
        cliBinary,
        "mcp",
        "--transport",
        "http",
        "--base-url",
        backendBaseUrl,
        "--session-id",
        "00000000-0000-4000-8000-000000000001",
      ],
      cwd: repoRoot,
      env: process.env as Record<string, string>,
    });
    const client = new Client({ name: "cli-e2e", version: "1.0.0" });

    await client.connect(transport);

    try {
      const toolList = await client.listTools();
      const names = toolList.tools.map((tool) => tool.name);
      expect(names).toEqual(
        expect.arrayContaining(["observe", "status", "move", "press", "execute_macro", "register_identity", "whoami"])
      );

      const observe = await client.callTool({
        name: "observe",
        arguments: { include_image: false },
      });
      const observeEntry = readFirstTextEntry(observe);
      expect(observeEntry?.type).toBe("text");
      expect(observeEntry?.text).toContain("HELLO SNAPSHOT");

      const move = await client.callTool({
        name: "move",
        arguments: { direction: "down", times: 1 },
      });
      const moveEntry = readFirstTextEntry(move);
      expect(moveEntry?.type).toBe("text");
      expect(moveEntry?.text).toContain("down");

      expect(
        requests.some(
          (request) =>
            request.path.startsWith("/api/mcp/tools?session_id=") &&
            (request.body as { tool?: string } | null)?.tool === "move"
        )
      ).toBe(true);
    } finally {
      await client.close();
      await transport.close();
    }
  });

  it("boots register and skill commands from the binary", async () => {
    const { stdout: registerOutput } = await runCliBinary(
      [
        "register",
        "--transport",
        "http",
        "--base-url",
        backendBaseUrl,
        "--session-id",
        "00000000-0000-4000-8000-000000000002",
        "--agent-id",
        "oak-lab-runner",
        "--identity-name",
        "trainer-oak",
      ],
    );
    expect(registerOutput).toContain('"sessionSecret": "secret-abc"');
    expect(
      requests.some(
          (request) =>
            request.path.startsWith("/api/mcp/tools?session_id=") &&
            (request.body as { tool?: string; input?: { name?: string } } | null)?.tool === "register_identity" &&
            (request.body as { input?: { name?: string } } | null)?.input?.name === "trainer-oak"
      )
    ).toBe(true);
    expect(
      requests.some(
        (request) =>
          request.path === "/api/arena/session-secret?session_id=00000000-0000-4000-8000-000000000002"
      )
    ).toBe(true);

    const skillOutput = execFileSync(
      process.execPath,
      [cliBinary, "skill", "--print"],
      {
        cwd: repoRoot,
        encoding: "utf8",
        env: process.env,
      }
    );
    expect(skillOutput).toContain("# Crystal LLM");
  });

  it("can drive the text UI in non-interactive test mode", async () => {
    const child = spawn(
      process.execPath,
      [
        cliBinary,
        "play",
        "--transport",
        "http",
        "--base-url",
        backendBaseUrl,
        "--session-id",
        "00000000-0000-4000-8000-000000000003",
      ],
      {
        cwd: repoRoot,
        env: {
          ...process.env,
          POKECRYSTAL_CLI_ALLOW_NON_TTY: "1",
          POKECRYSTAL_CLI_TEST_INK: "1",
        },
        stdio: ["pipe", "pipe", "pipe"],
      }
    );

    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });

    await waitFor(() => stdout.includes("PokeCrystal CLI"));
    child.stdin.write("z");
    await waitFor(() =>
      requests.some(
        (request) =>
          request.path.startsWith("/api/mcp/tools?session_id=") &&
          (request.body as { tool?: string } | null)?.tool === "press"
      )
    );
    child.stdin.write(":q!\r");

    const [exitCode] = (await once(child, "exit")) as [number | null];
    expect(exitCode).toBe(0);
    expect(stderr).not.toContain("Text UI requires an interactive TTY.");
    expect(stdout).toContain("HELLO SNAPSHOT");
  });
});
