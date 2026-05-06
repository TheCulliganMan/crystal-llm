import http from "node:http";
import { AddressInfo } from "node:net";
import { spawn, execFile } from "node:child_process";
import path from "node:path";

type LoggedRequest = {
  method: string;
  url: string;
  headers: http.IncomingHttpHeaders;
  body?: unknown;
};

type JsonRpcMessage = {
  jsonrpc?: string;
  id?: string | number | null;
  method?: string;
  params?: Record<string, unknown>;
  result?: unknown;
  error?: unknown;
};

const serializeJsonRpcMessage = (message: JsonRpcMessage): string => `${JSON.stringify(message)}\n`;

type FakeServer = {
  baseUrl: string;
  requests: LoggedRequest[];
  close: () => Promise<void>;
};

type FakeBackendScenario =
  | "basic"
  | "first-battle"
  | "battle-stall"
  | "battle-long-stall"
  | "battle-transition-wait"
  | "battle-observe-delay"
  | "battle-late-prompt"
  | "instant-wild-menu"
  | "instant-trainer-menu"
  | "npc-dialogue-close";

const readJsonBody = async (request: http.IncomingMessage): Promise<any> => {
  const chunks: Buffer[] = [];
  for await (const chunk of request) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  if (!chunks.length) {
    return {};
  }
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
};

const toolResponse = (text: string) => ({
  ok: true,
  result: {
    content: [{ type: "text", text }],
  },
});

const firstBattleSnapshotText = (
  state: { x: number; y: number; hasStarter: boolean; inBattle: boolean }
): string => {
  if (state.inBattle) {
    return "BATTLE\nA wild Pidgey appeared!\nFight with J/Z.";
  }
  if (state.hasStarter) {
    return `ROUTE 29\nStarter acquired. Walk to tall grass.\nX=${state.x} Y=${state.y}`;
  }
  return `ELM LAB\nChoose a starter.\nX=${state.x} Y=${state.y}`;
};

const isInstantBattleMenuScenario = (scenario: FakeBackendScenario): boolean =>
  scenario === "instant-wild-menu" || scenario === "instant-trainer-menu";

export const startFakeCliBackend = async (
  options: { scenario?: FakeBackendScenario } = {}
): Promise<FakeServer> => {
  const requests: LoggedRequest[] = [];
  let x = 7;
  let y = 4;
  const scenario = options.scenario ?? "basic";
  let hasStarter = scenario === "first-battle" ? false : true;
  let inBattle =
    scenario === "battle-stall" ||
    scenario === "battle-long-stall" ||
    scenario === "battle-transition-wait" ||
    scenario === "battle-observe-delay" ||
    scenario === "battle-late-prompt";
  let battleStallFrames = scenario === "battle-transition-wait" ? 18 : 0;
  let latePromptFrames = 0;
  let battleTurns = 0;
  let battleWon = false;
  let npcDialogueOpen = false;

  const server = http.createServer(async (request, response) => {
    const url = request.url ?? "/";
    if (request.method === "POST" && url.startsWith("/api/mcp/tools")) {
      const body = await readJsonBody(request);
      requests.push({
        method: request.method,
        url,
        headers: request.headers,
        body,
      });
      const name = body?.tool ?? body?.name;
      const input = body?.input ?? body?.arguments ?? {};
      if (name === "observe") {
        if (
          (scenario === "battle-stall" ||
            scenario === "battle-long-stall" ||
            scenario === "battle-transition-wait") &&
          inBattle &&
          battleStallFrames > 0
        ) {
          battleStallFrames -= 1;
          if (battleStallFrames <= 0) {
            inBattle = false;
            battleWon = true;
          }
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                battleStallFrames > 0
                  ? scenario === "battle-transition-wait"
                    ? "BATTLE\nBATTLE TRANSITION\nThe battle is starting...\nWait: battle intro animation"
                    : "BATTLE\nfx: busy\nrsn: busy"
                  : "BATTLE\nResolved."
              )
            )
          );
          return;
        }
        if (scenario === "battle-observe-delay" && inBattle && battleStallFrames > 0) {
          battleStallFrames -= 1;
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                battleStallFrames > 0
                  ? "BATTLE\nENEMY SENTRET L4 HP 16/16 STATUS OK ITEM NONE\nALLY  CYNDAQUIL L8 HP 26/26 STATUS OK ITEM NONE\nALLY EXP 406 NEXT 13\nWait: move animation sound delay"
                  : "BATTLE\nENEMY SENTRET L4 HP 7/16 STATUS OK ITEM NONE\nALLY  CYNDAQUIL L8 HP 26/26 STATUS OK ITEM NONE\nALLY EXP 406 NEXT 13\nA=Advance B=Close\n\nDIALOGUE\nCYNDAQUIL used\nTACKLE!"
              )
            )
          );
          return;
        }
        if (scenario === "first-battle") {
          const observeText = firstBattleSnapshotText({ x, y, hasStarter, inBattle });
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify(toolResponse(observeText)));
          return;
        }
        if (isInstantBattleMenuScenario(scenario)) {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                "BATTLE\nENEMY RATTATA L4 HP 12/12 STATUS OK ITEM NONE\nALLY  TOTODILE L8 HP 28/28 STATUS OK ITEM NONE\n\nMENU\nFIGHT  PKMN\nPACK   RUN"
              )
            )
          );
          return;
        }
        if (scenario === "battle-observe-delay") {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                "BATTLE\nENEMY SENTRET L4 HP 16/16 STATUS OK ITEM NONE\nALLY  CYNDAQUIL L8 HP 26/26 STATUS OK ITEM NONE\nALLY EXP 406 NEXT 13\n\nMENU\n▶ TACKLE (PP 35/35)\nLEER (PP 30/30)\nSMOKESCREEN (PP 20/20)\nCANCEL"
              )
            )
          );
          return;
        }
        if (
          scenario === "battle-stall" ||
          scenario === "battle-long-stall" ||
          scenario === "battle-transition-wait"
        ) {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                scenario === "battle-transition-wait"
                  ? "BATTLE\nBATTLE TRANSITION\nThe battle is starting...\nWait: battle intro animation"
                  : "BATTLE\nBusy: waiting for animation."
              )
            )
          );
          return;
        }
        if (scenario === "battle-late-prompt") {
          if (latePromptFrames > 0) {
            latePromptFrames -= 1;
            response.setHeader("content-type", "application/json");
            response.end(
              JSON.stringify(
                toolResponse(
                  latePromptFrames > 0
                    ? "BATTLE\nENEMY SENTRET L4 HP 16/16 STATUS OK ITEM NONE\nALLY  CYNDAQUIL L8 HP 26/26 STATUS OK ITEM NONE\nALLY EXP 406 NEXT 13"
                    : "BATTLE\nENEMY SENTRET L4 HP 7/16 STATUS OK ITEM NONE\nALLY  CYNDAQUIL L8 HP 26/26 STATUS OK ITEM NONE\nALLY EXP 406 NEXT 13\nA=Advance B=Close\n\nDIALOGUE\nCYNDAQUIL used\nTACKLE!"
                )
              )
            );
            return;
          }
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                "BATTLE\nENEMY SENTRET L4 HP 16/16 STATUS OK ITEM NONE\nALLY  CYNDAQUIL L8 HP 26/26 STATUS OK ITEM NONE\nALLY EXP 406 NEXT 13\n\nMENU\n▶ TACKLE (PP 35/35)\nLEER (PP 30/30)\nSMOKESCREEN (PP 20/20)\nCANCEL"
              )
            )
          );
          return;
        }
        if (scenario === "npc-dialogue-close") {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                npcDialogueOpen
                  ? "OVERWORLD\nROUTE30 BERRY HOUSE\nTalk to the berry man.\n\nDIALOGUE\nCheck trees for\nBERRIES."
                  : "OVERWORLD\nROUTE30 BERRY HOUSE\nTalk to the berry man."
              )
            )
          );
          return;
        }
        response.setHeader("content-type", "application/json");
        response.end(JSON.stringify(toolResponse(`TEST OBSERVE\nX=${x} Y=${y}`)));
        return;
      }
      if (name === "status") {
        if (scenario === "first-battle") {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                JSON.stringify({
                  mode: inBattle ? "battle" : "overworld",
                  map: inBattle ? "BATTLE" : hasStarter ? "Route29" : "ElmsLab",
                  coords: { x, y },
                  can_move: !inBattle,
                  party_summary: { count: hasStarter ? 1 : 0 },
                  battle_won: battleWon,
                })
              )
            )
          );
          return;
        }
        if (
          scenario === "battle-stall" ||
          scenario === "battle-long-stall" ||
          scenario === "battle-transition-wait" ||
          scenario === "battle-observe-delay"
        ) {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                JSON.stringify({
                  mode: inBattle ? "battle" : "overworld",
                  map: inBattle ? "BATTLE" : "Route29",
                  coords: { x, y },
                  can_move: !inBattle,
                  party_summary: { count: 1 },
                  battle_won: battleWon,
                })
              )
            )
          );
          return;
        }
        if (isInstantBattleMenuScenario(scenario)) {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                JSON.stringify({
                  mode: "battle",
                  instant_mode: true,
                  battle_is_trainer: scenario === "instant-trainer-menu",
                  map: "Route30",
                  can_move: false,
                  input_blocked_reason: null,
                })
              )
            )
          );
          return;
        }
        if (scenario === "battle-late-prompt") {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                JSON.stringify({
                  mode: "battle",
                  map: "Route29",
                  coords: { x, y },
                  can_move: false,
                  party_summary: { count: 1 },
                })
              )
            )
          );
          return;
        }
        if (scenario === "npc-dialogue-close") {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(
                JSON.stringify({
                  mode: "overworld",
                  map: "Route30BerryHouse",
                  coords: { x, y },
                  can_move: !npcDialogueOpen,
                  dialogue_open: npcDialogueOpen,
                })
              )
            )
          );
          return;
        }
        response.setHeader("content-type", "application/json");
        response.end(
          JSON.stringify(
            toolResponse(JSON.stringify({ mode: "overworld", map: "TEST MAP", coords: { x, y }, can_move: true }))
          )
        );
        return;
      }
      if (name === "recent_events") {
        if (
          scenario === "battle-stall" ||
          scenario === "battle-long-stall" ||
          scenario === "battle-transition-wait" ||
          scenario === "battle-observe-delay"
        ) {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse([
                "n: 1",
                `sum: "${
                  (scenario === "battle-stall" || scenario === "battle-transition-wait") && inBattle
                    ? "battle active"
                    : "latest @ " + x + "," + y
                }"`,
              ].join("\n"))
            )
          );
          return;
        }
        if (scenario === "battle-late-prompt") {
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify(toolResponse('n: 1\nsum: "battle active"')));
          return;
        }
        if (isInstantBattleMenuScenario(scenario)) {
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify(toolResponse('n: 1\nsum: "instant trainer battle menu"')));
          return;
        }
        if (scenario === "npc-dialogue-close") {
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify(
              toolResponse(`n: 1\nsum: "${npcDialogueOpen ? "prompt open" : "prompt closed"}"`)
            )
          );
          return;
        }
        response.setHeader("content-type", "application/json");
        response.end(
          JSON.stringify(
            toolResponse(
              [
                "n: 1",
                `sum: "${scenario === "first-battle" && inBattle ? "battle active" : `latest @ ${x},${y}`}"`,
              ].join("\n")
            )
          )
        );
        return;
      }
      if (name === "move") {
        if (isInstantBattleMenuScenario(scenario)) {
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify(toolResponse("ok: 1\nch: 1\nfx: busy\nrsn: busy\nBATTLE\nmenu moved")));
          return;
        }
        const direction = String(input?.direction ?? "up");
        if (direction === "up") y -= 1;
        if (direction === "down") y += 1;
        if (direction === "left") x -= 1;
        if (direction === "right") x += 1;
        if (scenario === "first-battle") {
          if (!hasStarter && x <= 5 && y <= 2) {
            hasStarter = true;
          }
          if (hasStarter && !inBattle && y >= 10) {
            inBattle = true;
          }
        }
        response.setHeader("content-type", "application/json");
        response.end(
          JSON.stringify(
            toolResponse(
              scenario === "first-battle"
                ? firstBattleSnapshotText({ x, y, hasStarter, inBattle })
                : `MOVED ${direction.toUpperCase()}\nX=${x} Y=${y}`
            )
          )
        );
        return;
      }
      if (name === "press") {
        if (isInstantBattleMenuScenario(scenario)) {
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify(toolResponse("ok: 1\nch: 1\nfx: busy\nrsn: busy\nBATTLE\nbutton accepted")));
          return;
        }
        if (
          scenario === "battle-stall" ||
          scenario === "battle-long-stall" ||
          scenario === "battle-transition-wait" ||
          scenario === "battle-observe-delay"
        ) {
          const button = String(input?.button ?? "a").toLowerCase();
          if (button === "a") {
            if (scenario === "battle-observe-delay") {
              battleStallFrames = 3;
              response.setHeader("content-type", "application/json");
              response.end(
                JSON.stringify(
                  toolResponse(
                    "ctx:\n  m: battle\n  last: menu closed @ Route29 105,25 | menu closed\na:\n  ok: 1\n  ch: 1\n  fx: changed\n  ev[1\t]: \"pressed:a:1\""
                  )
                )
              );
              return;
            }
            if (battleStallFrames === 0) {
              battleStallFrames =
                scenario === "battle-long-stall" || scenario === "battle-transition-wait" ? 18 : 2;
              response.setHeader("content-type", "application/json");
              response.end(
                JSON.stringify(
                  toolResponse(
                    scenario === "battle-transition-wait"
                      ? "BATTLE\nBATTLE TRANSITION\nThe battle is starting...\nWait: battle intro animation"
                      : "ok: 0\nch: 0\nfx: busy\nrsn: busy\nBATTLE\nA press ignored while frozen."
                  )
                )
              );
              return;
            }
            response.setHeader("content-type", "application/json");
            response.end(JSON.stringify(toolResponse("BATTLE\nPress accepted.")));
            return;
          }
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify(toolResponse("BATTLE\nunsupported button.")));
          return;
        }
        if (scenario === "battle-late-prompt") {
          const button = String(input?.button ?? "a").toLowerCase();
          if (button === "a") {
            latePromptFrames = 2;
            response.setHeader("content-type", "application/json");
            response.end(
              JSON.stringify(
                toolResponse(
                  "ctx:\n  m: battle\n  last: menu closed @ Route29 105,25 | menu closed\na:\n  ok: 1\n  ch: 1\n  fx: changed\n  ev[1\t]: \"pressed:a:1\""
                )
              )
            );
            return;
          }
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify(toolResponse("BATTLE\nunsupported button.")));
          return;
        }
        if (scenario === "npc-dialogue-close") {
          const button = String(input?.button ?? "a").toLowerCase();
          if (button === "a") {
            npcDialogueOpen = !npcDialogueOpen;
            response.setHeader("content-type", "application/json");
            response.end(
              JSON.stringify(
                toolResponse(
                  npcDialogueOpen
                    ? 'ctx:\n  m: overworld\n  dlg: 1\n  txt: 1\n  pr: 1\n  lock: 1\n  busy: 1\n  blk: dialogue\n  last: "prompt opened:dialogue @ Route30BerryHouse 3,7 | prompt opened:dialogue"\na:\n  ok: 1\n  ch: 1\n  fx: changed\n  ev[1\t]: "pressed:a:1"'
                    : 'ctx:\n  m: overworld\n  last: "prompt closed:dialogue @ Route30BerryHouse 3,7 | prompt closed:dialogue"\na:\n  ok: 1\n  ch: 1\n  fx: changed\n  ev[1\t]: "pressed:a:1"'
                )
              )
            );
            return;
          }
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify(toolResponse("OVERWORLD\nunsupported button.")));
          return;
        }
        if (scenario === "first-battle") {
          const button = String(input?.button ?? "a").toLowerCase();
          if (!hasStarter && button === "a") {
            hasStarter = true;
          } else if (inBattle && button === "a") {
            battleTurns += 1;
            if (battleTurns >= 3) {
              inBattle = false;
              battleWon = true;
            }
          }
        }
        response.setHeader("content-type", "application/json");
        response.end(
          JSON.stringify(
            toolResponse(
              scenario === "first-battle"
                ? firstBattleSnapshotText({ x, y, hasStarter, inBattle })
                : `PRESSED ${String(input?.button ?? "a").toUpperCase()}`
            )
          )
        );
        return;
      }
      if (name === "wait") {
        response.setHeader("content-type", "application/json");
        response.end(JSON.stringify(toolResponse(`WAITED ${String(input?.frames ?? 1)} FRAMES\nX=${x} Y=${y}`)));
        return;
      }
      if (name === "execute_macro") {
        response.setHeader("content-type", "application/json");
        response.end(JSON.stringify(toolResponse("DIALOG ADVANCED")));
        return;
      }
      if (name === "register_identity") {
        response.setHeader("content-type", "application/json");
        response.end(
          JSON.stringify(
            toolResponse(JSON.stringify({ token: "token-123", playerId: "player-1" }))
          )
        );
        return;
      }
      if (name === "whoami") {
        response.setHeader("content-type", "application/json");
        response.end(JSON.stringify(toolResponse(JSON.stringify({ playerId: "player-1" }))));
        return;
      }
      response.statusCode = 404;
      response.setHeader("content-type", "application/json");
      response.end(JSON.stringify({ ok: false, error: `Unknown tool: ${String(name)}` }));
      return;
    }

    if (request.method === "GET" && url.startsWith("/api/arena/session-secret")) {
      requests.push({
        method: request.method,
        url,
        headers: request.headers,
      });
      response.setHeader("content-type", "application/json");
      response.end(JSON.stringify({ sessionSecret: "secret-123" }));
      return;
    }

    response.statusCode = 404;
    response.end("not found");
  });

  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", () => resolve()));
  const address = server.address() as AddressInfo;

  return {
    baseUrl: `http://127.0.0.1:${address.port}`,
    requests,
    close: () =>
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

export const cliBinPath = (): string =>
  path.resolve(__dirname, "..", "dist", "bin", "pokecrystal-cli.js");

export const runCliCommand = async (
  args: string[],
  options: { env?: NodeJS.ProcessEnv } = {}
): Promise<{ stdout: string; stderr: string }> => {
  const nodeBin = process.execPath;
  const cliPath = cliBinPath();
  return new Promise((resolve, reject) => {
    execFile(nodeBin, [cliPath, ...args], { env: options.env }, (error, stdout, stderr) => {
      if (error) {
        reject(Object.assign(error, { stdout, stderr }));
        return;
      }
      resolve({ stdout, stderr });
    });
  });
};

export const spawnCliProcess = (args: string[]) => {
  const nodeBin = process.execPath;
  const cliPath = cliBinPath();
  return spawn(nodeBin, [cliPath, ...args], {
    stdio: ["pipe", "pipe", "pipe"],
  });
};

export const waitForOutput = async (
  getOutput: () => string,
  pattern: RegExp,
  timeoutMs = 5_000
): Promise<string> => {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const output = getOutput();
    if (pattern.test(output)) {
      return output;
    }
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  throw new Error(`Timed out waiting for output: ${pattern}`);
};

export const createJsonRpcStream = (child: ReturnType<typeof spawnCliProcess>) => {
  let stdout = "";
  let stderr = "";
  const messages: JsonRpcMessage[] = [];

  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");

  child.stdout.on("data", (chunk: string) => {
    stdout += chunk;
    let newlineIndex = stdout.indexOf("\n");
    while (newlineIndex >= 0) {
      const line = stdout.slice(0, newlineIndex).trim();
      stdout = stdout.slice(newlineIndex + 1);
      if (line) {
        messages.push(JSON.parse(line) as JsonRpcMessage);
      }
      newlineIndex = stdout.indexOf("\n");
    }
  });

  child.stderr.on("data", (chunk: string) => {
    stderr += chunk;
  });

  const send = (message: JsonRpcMessage): void => {
    child.stdin.write(serializeJsonRpcMessage(message));
  };

  const waitForMessage = async (
    predicate: (message: JsonRpcMessage) => boolean,
    timeoutMs = 5_000
  ): Promise<JsonRpcMessage> => {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      const match = messages.find(predicate);
      if (match) {
        return match;
      }
      await new Promise((resolve) => setTimeout(resolve, 25));
    }
    throw new Error(`Timed out waiting for MCP message. stderr=${stderr}`);
  };

  return {
    send,
    waitForMessage,
    getStderr: () => stderr,
  };
};
