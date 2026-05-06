import type { CliOptions } from "./types";

export const resolveTransport = (
  options: Pick<CliOptions, "transport" | "baseUrl">
): "local" | "http" => {
  if (options.transport === "local" || options.transport === "http") {
    return options.transport;
  }
  return options.baseUrl ? "http" : "local";
};
