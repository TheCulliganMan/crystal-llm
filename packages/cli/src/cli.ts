import fs from "node:fs";
import { helpText, parseArgs, skillPath } from "./args";
import { extractText, PokecrystalToolsClient } from "./client";
import { runMcpProxyServer } from "./mcp-server";
import { withResolvedSessionLogFile } from "./session-log";

export type CliRuntimeStreams = {
  stdin?: NodeJS.ReadStream;
  stdout?: Pick<NodeJS.WriteStream, "write">;
  stderr?: Pick<NodeJS.WriteStream, "write">;
};

export type CliRuntimeOptions = CliRuntimeStreams & {
  fetchImpl?: typeof fetch;
};

export const runCli = async (
  argv: string[],
  runtime: CliRuntimeOptions = {}
): Promise<void> => {
  const options = withResolvedSessionLogFile(parseArgs(argv));
  const stdout = runtime.stdout ?? process.stdout;
  const stderr = runtime.stderr ?? process.stderr;

  if (options.command === "help") {
    stdout.write(`${helpText()}\n`);
    return;
  }

  if (options.command === "skill") {
    const path = skillPath("SKILL.md");
    if (options.printSkill) {
      stdout.write(`${fs.readFileSync(path, "utf8")}\n`);
      return;
    }
    stdout.write(`${path}\n`);
    return;
  }

  if (options.command === "register") {
    const client = new PokecrystalToolsClient({
      ...options,
      fetchImpl: runtime.fetchImpl,
    });
    const result = await client.registerIdentity({
      agentId: options.agentId,
      identityName: options.identityName,
    });
    stdout.write(`${extractText(result.tool.content)}\n`);
    if (result.sessionSecret) {
      stdout.write(`${JSON.stringify({ sessionSecret: result.sessionSecret }, null, 2)}\n`);
    }
    return;
  }

  if (options.command === "mcp") {
    await runMcpProxyServer(options);
    return;
  }

  if (options.command === "play" || options.command === "play-recorded") {
    const { runTextUi } = await import("./tui");
    await runTextUi(options, {
      stdin: runtime.stdin,
      stdout,
      fetchImpl: runtime.fetchImpl,
    });
    return;
  }

  stderr.write(`${helpText()}\n`);
};
