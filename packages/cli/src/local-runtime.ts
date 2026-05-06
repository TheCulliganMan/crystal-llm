import fs from "node:fs";
import path from "node:path";
import Module from "node:module";
import { transformSync } from "@babel/core";
import type { CliOptions, ToolResult } from "./types";

type McpToolDefinition = {
  inputSchema: {
    safeParse: (input: unknown) =>
      | { success: true; data: Record<string, unknown> }
      | {
          success: false;
          error: { issues: Array<{ path?: Array<string | number>; message?: string }> };
        };
  };
  handler: (input: Record<string, unknown>, extra?: unknown) => Promise<ToolResult>;
};

type LocalRuntimeModules = {
  getMcpToolDefinition: (name: string) => McpToolDefinition | undefined;
  buildSessionSecret: (sessionId: string, playerId: string) => string;
};

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const webSrcRoot = path.join(repoRoot, "apps", "web", "src");
const emptyStubPath = path.join(repoRoot, "packages", "cli", "src", "stubs", "empty-module.js");
const moduleLoader = Module as typeof Module & {
  _resolveFilename: (
    request: string,
    parent: NodeModule | undefined,
    isMain: boolean,
    options?: unknown
  ) => string;
};

let hooksInstalled = false;
let cachedRuntime: LocalRuntimeModules | null = null;

const shouldTranspileTs = (filename: string): boolean => {
  const normalized = filename.replace(/\\/g, "/");
  return (
    normalized.startsWith(path.join(repoRoot, "apps", "web").replace(/\\/g, "/")) ||
    normalized.startsWith(path.join(repoRoot, "packages").replace(/\\/g, "/"))
  );
};

const resolveAlias = (request: string): string => {
  if (request === "server-only" || request === "client-only") {
    return emptyStubPath;
  }
  if (request === "@pokecrystal/core") {
    return path.join(repoRoot, "packages", "core", "src", "index.ts");
  }
  if (request.startsWith("@pokecrystal/core/")) {
    return path.join(repoRoot, "packages", "core", "src", request.slice("@pokecrystal/core/".length));
  }
  if (request.includes("/node_modules/@pokecrystal/core/src/")) {
    return request.replace("/node_modules/@pokecrystal/core/src/", "/packages/core/src/");
  }
  if (request === "@pokecrystal/assets") {
    return path.join(repoRoot, "packages", "assets", "src", "index.ts");
  }
  if (request.startsWith("@pokecrystal/assets/")) {
    return path.join(repoRoot, "packages", "assets", "src", request.slice("@pokecrystal/assets/".length));
  }
  if (request.includes("/node_modules/@pokecrystal/assets/src/")) {
    return request.replace("/node_modules/@pokecrystal/assets/src/", "/packages/assets/src/");
  }
  if (request === "@pokecrystal/exporters") {
    return path.join(repoRoot, "packages", "exporters", "src", "index.ts");
  }
  if (request.startsWith("@pokecrystal/exporters/")) {
    return path.join(repoRoot, "packages", "exporters", "src", request.slice("@pokecrystal/exporters/".length));
  }
  if (request.includes("/node_modules/@pokecrystal/exporters/src/")) {
    return request.replace("/node_modules/@pokecrystal/exporters/src/", "/packages/exporters/src/");
  }
  if (request.startsWith("@/")) {
    return path.join(webSrcRoot, request.slice(2));
  }
  return request;
};

const installRequireHooks = (): void => {
  if (hooksInstalled) {
    return;
  }
  hooksInstalled = true;

  const originalResolveFilename = moduleLoader._resolveFilename;
  moduleLoader._resolveFilename = function patchedResolveFilename(
    request: string,
    parent: NodeModule | undefined,
    isMain: boolean,
    options?: unknown
  ): string {
    return originalResolveFilename.call(this, resolveAlias(request), parent, isMain, options);
  };

  const compileTypeScript = (mod: NodeModule, filename: string): void => {
    const source = fs
      .readFileSync(filename, "utf8")
      .replace(/^\s*import\s+["']server-only["'];?\s*$/gm, "")
      .replace(/^\s*import\s+["']client-only["'];?\s*$/gm, "");
    const isTsx = filename.endsWith(".tsx");
    const transformed = transformSync(source, {
      filename,
      babelrc: false,
      configFile: false,
      sourceMaps: "inline",
      presets: [
        [
          "@babel/preset-env",
          {
            targets: { node: "current" },
            modules: "commonjs",
          },
        ],
        [
          "@babel/preset-typescript",
          {
            allowDeclareFields: true,
            allExtensions: true,
            isTSX: isTsx,
          },
        ],
        ...(isTsx
          ? [
              [
                "@babel/preset-react",
                {
                  runtime: "automatic",
                },
              ] as const,
            ]
          : []),
      ],
    });
    (mod as NodeModule & { _compile: (code: string, file: string) => void })._compile(
      transformed?.code ?? source,
      filename
    );
  };

  const registerExtension = (extension: ".ts" | ".tsx"): void => {
    require.extensions[extension] = (mod, filename) => {
      if (!shouldTranspileTs(filename)) {
        throw new Error(`Refusing to transpile unexpected TypeScript file outside repo workspaces: ${filename}`);
      }
      compileTypeScript(mod, filename);
    };
  };

  registerExtension(".ts");
  registerExtension(".tsx");
};

const loadRuntimeModules = (): LocalRuntimeModules => {
  if (cachedRuntime) {
    return cachedRuntime;
  }
  installRequireHooks();
  const registryPath = path.join(webSrcRoot, "app", "api", "[transport]", "tools", "registry.ts");
  const sessionGuardsPath = path.join(webSrcRoot, "app", "mcp", "session-guards.ts");
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  const { getMcpToolDefinition } = require(registryPath) as Pick<LocalRuntimeModules, "getMcpToolDefinition">;
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  const { buildSessionSecret } = require(sessionGuardsPath) as Pick<LocalRuntimeModules, "buildSessionSecret">;
  cachedRuntime = { getMcpToolDefinition, buildSessionSecret };
  return cachedRuntime;
};

const buildHeaders = (
  options: Pick<CliOptions, "sessionId" | "sessionMode" | "token" | "sessionSecret">
): Record<string, string> => {
  const headers: Record<string, string> = {
    "mcp-session-id": options.sessionId,
    "x-mcp-session": options.sessionId,
  };
  if (options.sessionMode) {
    headers["x-pokecrystal-session-mode"] = options.sessionMode;
  }
  if (options.token) {
    headers.authorization = `Bearer ${options.token}`;
  }
  if (options.sessionSecret) {
    headers["x-session-secret"] = options.sessionSecret;
  }
  return headers;
};

export const callLocalTool = async (
  options: Pick<CliOptions, "sessionId" | "sessionMode" | "token" | "sessionSecret">,
  name: string,
  input: Record<string, unknown> = {}
): Promise<ToolResult> => {
  const runtime = loadRuntimeModules();
  const definition = runtime.getMcpToolDefinition(name);
  if (!definition) {
    throw new Error(`Unknown tool: ${name}`);
  }
  const parsed = definition.inputSchema.safeParse(input);
  if (!parsed.success) {
    const issue = parsed.error.issues[0];
    const pathText = issue?.path?.length ? ` (${issue.path.join(".")})` : "";
    throw new Error(`Invalid tool arguments${pathText}: ${issue?.message ?? "schema validation failed"}`);
  }
  return definition.handler(parsed.data, {
    requestInfo: {
      headers: buildHeaders(options),
    },
    rawInput: input,
  });
};

export const buildLocalSessionSecret = (sessionId: string, playerId: string): string => {
  return loadRuntimeModules().buildSessionSecret(sessionId, playerId);
};
