#!/usr/bin/env node
import fs from "node:fs";
import { helpText, parseArgs, skillPath } from "../args";
import { extractText, PokecrystalToolsClient } from "../client";
import { runMcpProxyServer } from "../mcp-server";

const main = async (): Promise<void> => {
  const options = parseArgs(process.argv.slice(2));

  if (options.command === "help") {
    process.stdout.write(`${helpText()}\n`);
    return;
  }

  if (options.command === "skill") {
    const path = skillPath("SKILL.md");
    if (options.printSkill) {
      process.stdout.write(fs.readFileSync(path, "utf8"));
      return;
    }
    process.stdout.write(`${path}\n`);
    return;
  }

  if (options.command === "register") {
    const client = new PokecrystalToolsClient(options);
    const result = await client.registerIdentity({
      agentId: options.agentId,
      identityName: options.identityName,
    });
    process.stdout.write(`${extractText(result.tool.content)}\n`);
    if (result.sessionSecret) {
      process.stdout.write(`${JSON.stringify({ sessionSecret: result.sessionSecret }, null, 2)}\n`);
    }
    return;
  }

  if (options.command === "mcp") {
    await runMcpProxyServer(options);
    return;
  }

  if (options.command === "play" || options.command === "play-recorded") {
    const { runTextUi } = await import("../tui");
    await runTextUi(options);
  }
};

void main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`${message}\n`);
  process.exit(1);
});
