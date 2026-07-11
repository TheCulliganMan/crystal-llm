import { AsyncLocalStorage } from "node:async_hooks";

export type McpIdentityContextValue = {
  playerId: string;
  token: string;
  name?: string | null;
};

const identityContext = new AsyncLocalStorage<McpIdentityContextValue | null>();

export const getMcpIdentityContext = (): McpIdentityContextValue | null =>
  identityContext.getStore() ?? null;

export const runWithMcpIdentityContext = <T>(
  identity: McpIdentityContextValue | null,
  fn: () => T
): T => identityContext.run(identity, fn);
