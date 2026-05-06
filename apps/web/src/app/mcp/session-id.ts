export const PRIMARY_MCP_SESSION_ID = "ultimate-run";
const SESSION_ID_REGEX = /^[a-zA-Z0-9_-]{1,64}$/;

export const resolveMcpSessionId = (candidate?: string | null): string => {
  const trimmed = candidate?.trim();
  if (trimmed && SESSION_ID_REGEX.test(trimmed)) {
    return trimmed;
  }
  return PRIMARY_MCP_SESSION_ID;
};
