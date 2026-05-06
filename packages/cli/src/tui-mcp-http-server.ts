import http from "node:http";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { createMcpProxyServer } from "./mcp-server";
import type { CliOptions } from "./types";

export type LocalMcpHttpServer = {
  url: string;
  close: () => Promise<void>;
};

export const startLocalMcpHttpServer = async (options: CliOptions): Promise<LocalMcpHttpServer> => {
  const proxyOptions = {
    ...options,
    command: "mcp",
    toolsUrl: options.transport === "http" ? options.toolsUrl : undefined,
  } as CliOptions;

  const httpServer = http.createServer((request, response) => {
    if (!request.url?.startsWith("/mcp")) {
      response.statusCode = 404;
      response.end("Not found");
      return;
    }
    void (async () => {
      const server = createMcpProxyServer(proxyOptions);
      const transport = new StreamableHTTPServerTransport({
        sessionIdGenerator: undefined,
        enableJsonResponse: true,
      });
      await server.connect(transport);
      try {
        await transport.handleRequest(request, response);
      } finally {
        await transport.close();
        await server.close();
      }
    })().catch((error: unknown) => {
        if (!response.headersSent) {
          response.statusCode = 500;
        }
        response.end(error instanceof Error ? error.message : String(error));
      });
  });

  await new Promise<void>((resolve, reject) => {
    httpServer.once("error", reject);
    httpServer.listen(0, "127.0.0.1", () => {
      httpServer.off("error", reject);
      resolve();
    });
  });
  const address = httpServer.address();
  if (!address || typeof address === "string") {
    throw new Error("Failed to bind local MCP HTTP server.");
  }
  const url = `http://127.0.0.1:${address.port}/mcp?session_id=${encodeURIComponent(options.sessionId)}`;
  return {
    url,
    close: async () => {
      await new Promise<void>((resolve, reject) => {
        httpServer.close((error) => {
          if (error) reject(error);
          else resolve();
        });
      });
    },
  };
};
