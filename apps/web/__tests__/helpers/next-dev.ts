import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import net from "node:net";

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const findOpenPort = async (): Promise<number> =>
  new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") {
          resolve(address.port);
        } else {
          reject(new Error("Failed to acquire a free port."));
        }
      });
    });
  });

const waitForReady = async (
  mcpUrl: string,
  proc: ChildProcessWithoutNullStreams,
  getOutput: () => string,
  timeoutMs = 45_000
): Promise<void> => {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (proc.exitCode !== null) {
      const output = getOutput();
      const details = output ? `\n\nnext dev output:\n${output}` : "";
      throw new Error(`next dev exited before the MCP endpoint was ready.${details}`);
    }
    try {
      const res = await fetch(mcpUrl, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: "{}",
      });
      res.body?.cancel();
      // If the route isn't registered yet (or a different server is running),
      // Next will often respond 404. Keep waiting in that case.
      if (res.status !== 404) {
        return;
      }
    } catch {
      await sleep(500);
    }
  }
  throw new Error("Timed out waiting for next dev to start.");
};

const probeMcp = async (mcpUrl: string): Promise<boolean> => {
  try {
    const res = await fetch(mcpUrl, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: "{}",
    });
    res.body?.cancel();
    return res.status !== 404;
  } catch {
    return false;
  }
};

export async function startNextDev(port?: number): Promise<{
  proc: ChildProcessWithoutNullStreams | null;
  baseUrl: string;
  mcpUrl: string;
  port: number;
}> {
  const existingPort = port ?? (process.env.MCP_TEST_PORT ? Number(process.env.MCP_TEST_PORT) : 3000);
  const existingBaseUrl = process.env.MCP_TEST_BASE_URL ?? `http://localhost:${existingPort}`;
  const existingMcpUrl = `${existingBaseUrl}/api/mcp`;
  if (await probeMcp(existingMcpUrl)) {
    return { proc: null, baseUrl: existingBaseUrl, mcpUrl: existingMcpUrl, port: existingPort };
  }

  const selectedPort = port ?? (await findOpenPort());
  const nextBin = require.resolve("next/dist/bin/next");
  const proc: ChildProcessWithoutNullStreams = spawn(
    process.execPath,
    // Next.js 16 defaults to Turbopack, but this repo relies on a custom webpack
    // config (client shims). Make webpack explicit so `next dev` doesn't exit.
    [nextBin, "dev", "--webpack", "-p", String(selectedPort)],
    {
      cwd: process.cwd(),
      env: {
        ...process.env,
        PORT: String(selectedPort),
        NEXT_TELEMETRY_DISABLED: "1",
      },
      stdio: "pipe",
    }
  );

  let output = "";
  const appendOutput = (chunk: Buffer) => {
    output += chunk.toString();
    if (output.length > 4000) {
      output = output.slice(-4000);
    }
  };
  proc.stdout.on("data", appendOutput);
  proc.stderr.on("data", appendOutput);

  const baseUrl = `http://localhost:${selectedPort}`;
  const mcpUrl = `${baseUrl}/api/mcp`;

  await waitForReady(mcpUrl, proc, () => output);

  return { proc, baseUrl, mcpUrl, port: selectedPort };
}

export async function stopNextDev(proc?: ChildProcessWithoutNullStreams | null): Promise<void> {
  if (!proc || proc.killed) {
    return;
  }
  const detach = () => {
    proc.stdout.removeAllListeners();
    proc.stderr.removeAllListeners();
    proc.stdin.removeAllListeners();
    proc.stdout.destroy();
    proc.stderr.destroy();
    proc.stdin.end();
  };
  await new Promise<void>((resolve) => {
    const timeout = setTimeout(() => {
      proc.kill("SIGKILL");
      detach();
      resolve();
    }, 5_000);
    timeout.unref?.();
    proc.once("exit", () => {
      clearTimeout(timeout);
      detach();
      resolve();
    });
    proc.kill("SIGTERM");
  });
}
