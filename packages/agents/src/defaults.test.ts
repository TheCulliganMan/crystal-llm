import {
  DEFAULT_AGENT_MODEL,
  DEFAULT_OPENAI_MODEL,
  DEFAULT_OPENAI_MODEL_ID,
  DEFAULT_OLLAMA_MODEL,
  DEFAULT_OLLAMA_MODEL_ID,
  getDefaultAnthropicApiKey,
  getDefaultAnthropicBaseUrl,
  getDefaultGoogleApiBaseUrl,
  getDefaultGoogleApiKey,
  getDefaultOllamaApiBaseUrl,
  getDefaultOpenAIApiKey,
  getDefaultOpenAIBaseUrl,
  normalizeAgentModel,
  resolveMastraModel,
  resolveMastraProviderOptions,
} from "./defaults.js";
import { createTaskmasterMemory } from "./memory.js";
import { runnerInputSchema } from "./types.js";

describe("agent defaults", () => {
  afterEach(() => {
    delete process.env.LLAMA_CPP_BASE_URL;
    delete process.env.OLLAMA_BASE_URL;
    delete process.env.OLLAMA_API_KEY;
    delete process.env.OPENAI_BASE_URL;
    delete process.env.OPENAI_API_KEY;
    delete process.env.ANTHROPIC_BASE_URL;
    delete process.env.ANTHROPIC_API_KEY;
    delete process.env.GOOGLE_GENERATIVE_AI_BASE_URL;
    delete process.env.GOOGLE_GENERATIVE_AI_API_KEY;
    delete process.env.GEMINI_API_KEY;
  });

  it("defaults both agents to gpt-5.4 while keeping the ollama fallback available", () => {
    const parsed = runnerInputSchema.parse({
      session: {
        sessionId: "session-123",
      },
      immediateGoal: "Leave the bedroom.",
    });

    expect(DEFAULT_OPENAI_MODEL_ID).toBe("gpt-5.4");
    expect(DEFAULT_OPENAI_MODEL).toBe("openai/gpt-5.4");
    expect(DEFAULT_AGENT_MODEL).toBe(DEFAULT_OPENAI_MODEL);
    expect(DEFAULT_OLLAMA_MODEL_ID).toBe("gemma-4-E4B-it-Q4_K_M.gguf");
    expect(parsed.taskmasterModel).toBe(DEFAULT_OPENAI_MODEL);
    expect(parsed.playerModel).toBe(DEFAULT_OPENAI_MODEL);
  });

  it("uses the same default agent model for observational memory", () => {
    const memory = createTaskmasterMemory({
      storage: {
        get: jest.fn(),
        set: jest.fn(),
        delete: jest.fn(),
      } as never,
    });

    expect(memory).toBeDefined();
  });

  it("resolves ollama config only when the required env is present", () => {
    process.env.LLAMA_CPP_BASE_URL = "http://127.0.0.1:8080/";
    process.env.OLLAMA_API_KEY = "ollama-key";

    expect(resolveMastraModel(DEFAULT_OLLAMA_MODEL)).toEqual({
      id: DEFAULT_OLLAMA_MODEL,
      url: getDefaultOllamaApiBaseUrl(),
      apiKey: "ollama-key",
    });
  });

  it("throws when ollama env is missing", () => {
    expect(() => resolveMastraModel(DEFAULT_OLLAMA_MODEL)).toThrow(
      "LLAMA_CPP_BASE_URL or OLLAMA_BASE_URL must be set for ollama models."
    );
  });

  it("normalizes codex aliases into the Mastra openai provider path", () => {
    expect(normalizeAgentModel("codex/gpt-5.4")).toBe("codex/gpt-5.4");
    expect(normalizeAgentModel("openai-codex/gpt-5.4")).toBe("codex/gpt-5.4");
    expect(normalizeAgentModel("codex")).toBe("codex/gpt-5.4");
    expect(normalizeAgentModel("openai-codex")).toBe("codex/gpt-5.4");
  });

  it("returns normalized model refs directly from resolveMastraModel", () => {
    expect(resolveMastraModel("codex/gpt-5.4")).toBe("codex/gpt-5.4");
    expect(resolveMastraModel("openai-codex/gpt-5.4")).toBe("codex/gpt-5.4");
  });

  it("prefers the llama.cpp base URL when present", () => {
    process.env.LLAMA_CPP_BASE_URL = "http://127.0.0.1:8080/";
    process.env.OLLAMA_BASE_URL = "http://localhost:11434";
    process.env.OLLAMA_API_KEY = "ollama-key";

    expect(getDefaultOllamaApiBaseUrl()).toBe("http://127.0.0.1:8080/v1");
  });

  it("adds llama.cpp chat-template provider options for ollama models", () => {
    process.env.LLAMA_CPP_BASE_URL = "http://127.0.0.1:8080/";

    expect(resolveMastraProviderOptions(DEFAULT_OLLAMA_MODEL)).toEqual({
      ollama: {
        chat_template_kwargs: {
          enable_thinking: false,
        },
      },
    });
    expect(resolveMastraProviderOptions(DEFAULT_OPENAI_MODEL)).toBeUndefined();
  });

  it("does not add llama.cpp-only provider options for plain ollama hosts", () => {
    process.env.OLLAMA_BASE_URL = "http://localhost:11434";

    expect(resolveMastraProviderOptions(DEFAULT_OLLAMA_MODEL)).toBeUndefined();
  });

  it("resolves direct provider API configuration from env", () => {
    process.env.OPENAI_BASE_URL = "https://openai.example/v1/";
    process.env.OPENAI_API_KEY = "openai-key";
    process.env.ANTHROPIC_BASE_URL = "https://anthropic.example/";
    process.env.ANTHROPIC_API_KEY = "anthropic-key";
    process.env.GOOGLE_GENERATIVE_AI_BASE_URL = "https://google.example/v1beta/";
    process.env.GEMINI_API_KEY = "gemini-key";

    expect(getDefaultOpenAIBaseUrl()).toBe("https://openai.example/v1");
    expect(getDefaultOpenAIApiKey()).toBe("openai-key");
    expect(getDefaultAnthropicBaseUrl()).toBe("https://anthropic.example");
    expect(getDefaultAnthropicApiKey()).toBe("anthropic-key");
    expect(getDefaultGoogleApiBaseUrl()).toBe("https://google.example/v1beta");
    expect(getDefaultGoogleApiKey()).toBe("gemini-key");
  });
});
