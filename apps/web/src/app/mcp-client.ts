"use client";

export type McpToolResult = {
  content?: Array<{
    type: string;
    text?: string;
    data?: string;
    mimeType?: string;
  }>;
  isError?: boolean;
};

type CallMcpToolOptions = {
  baseUrl?: string;
  headers?: Record<string, string>;
};

const normalizeToolUrl = (baseUrl: string): string => {
  try {
    const origin = typeof window === "undefined" ? "http://localhost" : window.location.origin;
    const url = new URL(baseUrl, origin);
    if (url.pathname.endsWith("/api/mcp")) {
      url.pathname = `${url.pathname.replace(/\/+$/, "")}/tools`;
    }
    if (typeof window !== "undefined" && url.origin === window.location.origin) {
      return `${url.pathname}${url.search}`;
    }
    return url.toString();
  } catch {
    return baseUrl.endsWith("/api/mcp") ? `${baseUrl}/tools` : baseUrl;
  }
};

export const callMcpTool = async (
  name: string,
  args: Record<string, unknown> = {},
  options: CallMcpToolOptions = {}
): Promise<McpToolResult> => {
  const baseUrl = normalizeToolUrl(options.baseUrl ?? "/api/mcp");
  const response = await fetch(baseUrl, {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/json",
      ...options.headers,
    },
    body: JSON.stringify({
      tool: name,
      input: args,
    }),
  });
  const payload = (await response.json()) as {
    ok?: boolean;
    error?: string;
    result?: McpToolResult;
  };
  if (!response.ok || payload.ok === false) {
    throw new Error(payload.error ?? `MCP tool call failed (${response.status})`);
  }
  return payload.result ?? { content: [] };
};
