import { createDefaultMastraStorage, createTaskmasterMemoryScope } from "./memory.js";

describe("memory helpers", () => {
  it("derives stable taskmaster memory identifiers from the session", () => {
    expect(
      createTaskmasterMemoryScope({
        baseUrl: "http://127.0.0.1:3000",
        sessionId: "session-123",
        agentId: "agent-123",
        identityName: "trainer-agent-123",
        token: "token",
        sessionSecret: "secret",
      }),
    ).toEqual({
      resource: "playthrough:agent-123",
      thread: "taskmaster:session-123",
    });
  });

  it("can isolate one taskmaster batch from another with a batch suffix", () => {
    expect(
      createTaskmasterMemoryScope(
        {
          baseUrl: "http://127.0.0.1:3000",
          sessionId: "session-123",
          agentId: "agent-123",
          identityName: "trainer-agent-123",
          token: "token",
          sessionSecret: "secret",
        },
        { batchId: "batch-456" },
      ),
    ).toEqual({
      resource: "playthrough:agent-123",
      thread: "taskmaster:session-123-batch-456",
    });
  });

  it("creates a default libsql-backed storage instance", () => {
    const storage = createDefaultMastraStorage();
    expect(storage).toBeDefined();
  });

  it("uses the stable thread scope by default so memory carries across batches", () => {
    const first = createTaskmasterMemoryScope({
      baseUrl: "http://127.0.0.1:3000",
      sessionId: "session-123",
      agentId: "agent-123",
      identityName: "trainer-agent-123",
      token: "token",
      sessionSecret: "secret",
    });
    const second = createTaskmasterMemoryScope({
      baseUrl: "http://127.0.0.1:3000",
      sessionId: "session-123",
      agentId: "agent-123",
      identityName: "trainer-agent-123",
      token: "token",
      sessionSecret: "secret",
    });

    expect(second).toEqual(first);
    expect(second.thread).toBe("taskmaster:session-123");
  });
});
