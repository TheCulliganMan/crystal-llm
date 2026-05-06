export const DEFAULT_OPENAI_MODEL_ID = "gpt-5.4";
export const DEFAULT_OPENAI_MODEL = `openai/${DEFAULT_OPENAI_MODEL_ID}`;
export const DEFAULT_OLLAMA_MODEL_ID = "gemma-4-E4B-it-Q4_K_M.gguf";
export const DEFAULT_OLLAMA_MODEL = `ollama/${DEFAULT_OLLAMA_MODEL_ID}`;
export const DEFAULT_AGENT_MODEL = DEFAULT_OPENAI_MODEL;

const normalizeBaseUrl = (value: string): string => value.replace(/\/+$/, "");

export function getDefaultOllamaBaseUrl(): string {
  const configured = process.env.LLAMA_CPP_BASE_URL?.trim() || process.env.OLLAMA_BASE_URL?.trim();
  if (!configured) {
    throw new Error("LLAMA_CPP_BASE_URL or OLLAMA_BASE_URL must be set for ollama models.");
  }
  return configured;
}

export function getDefaultOllamaApiBaseUrl(): string {
  const normalized = normalizeBaseUrl(getDefaultOllamaBaseUrl());
  return normalized.endsWith("/v1") ? normalized : `${normalized}/v1`;
}

export function getDefaultAzureOpenAIEndpoint(): string {
  const configured = process.env.AZURE_OPENAI_ENDPOINT?.trim() || process.env.AZURE_ENDPOINT?.trim();
  if (!configured) {
    throw new Error("AZURE_OPENAI_ENDPOINT or AZURE_ENDPOINT must be set for azure-openai models.");
  }
  return normalizeBaseUrl(configured);
}

export function getDefaultAzureOpenAIApiKey(): string {
  const configured = process.env.AZURE_OPENAI_API_KEY?.trim() || process.env.AZURE_API_KEY?.trim();
  if (!configured) {
    throw new Error("AZURE_OPENAI_API_KEY or AZURE_API_KEY must be set for azure-openai models.");
  }
  return configured;
}

export function getDefaultAzureOpenAIApiVersion(): string {
  return process.env.AZURE_OPENAI_API_VERSION?.trim() || "2025-04-01-preview";
}

export function getDefaultOpenAIBaseUrl(): string {
  return normalizeBaseUrl(process.env.OPENAI_BASE_URL?.trim() || "https://api.openai.com/v1");
}

export function getDefaultOpenAIApiKey(): string {
  const configured = process.env.OPENAI_API_KEY?.trim();
  if (!configured) {
    throw new Error("OPENAI_API_KEY must be set for openai-direct models.");
  }
  return configured;
}

export function getDefaultAnthropicBaseUrl(): string {
  return normalizeBaseUrl(process.env.ANTHROPIC_BASE_URL?.trim() || "https://api.anthropic.com");
}

export function getDefaultAnthropicApiKey(): string {
  const configured = process.env.ANTHROPIC_API_KEY?.trim();
  if (!configured) {
    throw new Error("ANTHROPIC_API_KEY must be set for anthropic models.");
  }
  return configured;
}

export function getDefaultAnthropicApiVersion(): string {
  return process.env.ANTHROPIC_API_VERSION?.trim() || "2023-06-01";
}

export function getDefaultGoogleApiKey(): string {
  const configured = process.env.GOOGLE_GENERATIVE_AI_API_KEY?.trim() || process.env.GEMINI_API_KEY?.trim();
  if (!configured) {
    throw new Error("GOOGLE_GENERATIVE_AI_API_KEY or GEMINI_API_KEY must be set for google/gemini models.");
  }
  return configured;
}

export function getDefaultGoogleApiBaseUrl(): string {
  return normalizeBaseUrl(process.env.GOOGLE_GENERATIVE_AI_BASE_URL?.trim() || "https://generativelanguage.googleapis.com/v1beta");
}

export function resolveMastraProviderOptions(model: string):
  | { ollama: { chat_template_kwargs: { enable_thinking: false } } }
  | undefined {
  const normalizedModel = normalizeAgentModel(model);
  if (!normalizedModel.startsWith("ollama/") || !process.env.LLAMA_CPP_BASE_URL?.trim()) {
    return undefined;
  }

  return {
    ollama: {
      chat_template_kwargs: {
        enable_thinking: false,
      },
    },
  };
}

export function normalizeAgentModel(model: string): string {
  const trimmed = model.trim();
  if (!trimmed) {
    return DEFAULT_AGENT_MODEL;
  }

  const lower = trimmed.toLowerCase();
  if (lower === "codex" || lower === "openai-codex") {
    return `codex/${DEFAULT_OPENAI_MODEL_ID}`;
  }
  if (lower.startsWith("codex/")) {
    return trimmed;
  }
  if (lower.startsWith("openai-codex/")) {
    return `codex/${trimmed.slice("openai-codex/".length) || DEFAULT_OPENAI_MODEL_ID}`;
  }

  return trimmed;
}

export function resolveMastraModel(model: string): string | {
  id: `${string}/${string}`;
  url: string;
  apiKey: string;
} {
  const normalizedModel = normalizeAgentModel(model);
  if (!normalizedModel.startsWith("ollama/")) {
    return normalizedModel;
  }

  return {
    id: normalizedModel as `${string}/${string}`,
    url: getDefaultOllamaApiBaseUrl(),
    apiKey: process.env.OLLAMA_API_KEY?.trim() || (() => {
      throw new Error("OLLAMA_API_KEY must be set for ollama models.");
    })(),
  };
}
