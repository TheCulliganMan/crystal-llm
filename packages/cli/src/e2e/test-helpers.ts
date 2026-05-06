import { PassThrough } from "node:stream";
import { serializeMessage } from "@modelcontextprotocol/sdk/shared/stdio.js";
import type { JSONRPCMessage } from "@modelcontextprotocol/sdk/types.js";

export type FetchCall = {
  url: string;
  init?: RequestInit;
  body?: unknown;
};

export const createJsonResponse = (payload: unknown, status = 200): Response =>
  new Response(JSON.stringify(payload), {
    status,
    headers: {
      "content-type": "application/json",
    },
  });

export const createFakeFetch = (
  handlers: Record<string, (url: URL, init?: RequestInit, body?: unknown) => Response>
): {
  fetch: typeof globalThis.fetch;
  calls: FetchCall[];
} => {
  const calls: FetchCall[] = [];
  const fakeFetch: typeof globalThis.fetch = async (input: string | URL | Request, init?: RequestInit) => {
    const url = new URL(String(input));
    const body = typeof init?.body === "string" ? JSON.parse(init.body) : undefined;
    calls.push({ url: url.toString(), init, body });
    const handler = handlers[url.pathname] ?? handlers["*"];
    if (!handler) {
      return createJsonResponse({ ok: false, error: `Unexpected URL: ${url.pathname}` }, 500);
    }
    return handler(url, init, body);
  };
  return { fetch: fakeFetch, calls };
};

export class FakeTtyInput extends PassThrough {
  public isTTY = true;
  private rawMode = false;

  setRawMode(mode: boolean): void {
    this.rawMode = mode;
  }

  getRawMode(): boolean {
    return this.rawMode;
  }
}

export class FakeTtyOutput extends PassThrough {
  public isTTY = true;
  private chunks: string[] = [];

  write(chunk: unknown): boolean {
    this.chunks.push(Buffer.isBuffer(chunk) ? chunk.toString("utf8") : String(chunk));
    return super.write(chunk);
  }

  readText(): string {
    return this.chunks.join("");
  }
}

export const collectStreamText = async (stream: PassThrough): Promise<string> => {
  const chunks: string[] = [];
  stream.on("data", (chunk: Buffer | string) => {
    chunks.push(Buffer.isBuffer(chunk) ? chunk.toString("utf8") : String(chunk));
  });
  await new Promise((resolve) => setImmediate(resolve));
  return chunks.join("");
};

export const createInMemoryJsonRpcHarness = () => {
  const input = new PassThrough();
  const output = new PassThrough();
  const messages: JSONRPCMessage[] = [];
  let buffer = "";
  output.on("data", (chunk: Buffer | string) => {
    buffer += Buffer.isBuffer(chunk) ? chunk.toString("utf8") : String(chunk);
    let index = buffer.indexOf("\n");
    while (index !== -1) {
      const line = buffer.slice(0, index).trim();
      buffer = buffer.slice(index + 1);
      if (line) {
        messages.push(JSON.parse(line) as JSONRPCMessage);
      }
      index = buffer.indexOf("\n");
    }
  });
  return {
    input,
    output,
    messages,
    send(message: JSONRPCMessage): void {
      input.write(serializeMessage(message));
    },
    async nextMessage(predicate: (message: JSONRPCMessage) => boolean): Promise<JSONRPCMessage> {
      const existing = messages.find(predicate);
      if (existing) {
        return existing;
      }
      return await new Promise<JSONRPCMessage>((resolve) => {
        const timer = setInterval(() => {
          const found = messages.find(predicate);
          if (found) {
            clearInterval(timer);
            resolve(found);
          }
        }, 5);
      });
    },
  };
};
