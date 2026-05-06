import { BackendAdapter } from "./api";
import { WebBackend } from "./web-backend";

const BACKEND_ALIASES: Record<string, "web"> = {
  web: "web",
  canvas: "web",
  browser: "web",
  headless: "web",
  offscreen: "web",
};

function normalizeBackendName(name: string | null | undefined): "web" {
  const normalized = (name ?? "").trim().toLowerCase();
  if (!normalized) {
    return "web";
  }
  const resolved = BACKEND_ALIASES[normalized];
  if (!resolved) {
    throw new Error(`Unknown backend '${name}'.`);
  }
  return resolved;
}

export function listBackends(): string[] {
  return ["web"];
}

export function getBackend(name?: string | null): BackendAdapter {
  normalizeBackendName(name);
  const requested = (name ?? "").trim().toLowerCase();
  const headless = ["headless", "offscreen"].includes(requested);
  return new WebBackend({ headless });
}

