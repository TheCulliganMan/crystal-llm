import {
  buildFirehoseRecord,
  encodeFirehoseRecords,
  FIREHOSE_LIMIT_DEFAULT,
  FIREHOSE_LIMIT_MAX,
  parseFirehoseQuery,
} from "./training-firehose";

describe("training firehose helpers", () => {
  it("parses default firehose query params", () => {
    const url = new URL("http://localhost/api/arena/firehose");
    const parsed = parseFirehoseQuery(url);
    expect(parsed).toEqual({
      afterId: 0,
      limit: FIREHOSE_LIMIT_DEFAULT,
      runId: undefined,
      label: undefined,
    });
  });

  it("parses explicit firehose query params", () => {
    const url = new URL(
      "http://localhost/api/arena/firehose?after_id=12&limit=25&run_id=run-1&label=snapshot"
    );
    const parsed = parseFirehoseQuery(url);
    expect(parsed).toEqual({
      afterId: 12,
      limit: 25,
      runId: "run-1",
      label: "snapshot",
    });
  });

  it("rejects invalid numeric params", () => {
    const url = new URL("http://localhost/api/arena/firehose?after_id=-1");
    expect(() => parseFirehoseQuery(url)).toThrow("Invalid after_id.");
  });

  it("rejects too-large limits", () => {
    const url = new URL(`http://localhost/api/arena/firehose?limit=${FIREHOSE_LIMIT_MAX + 1}`);
    expect(() => parseFirehoseQuery(url)).toThrow("Invalid limit.");
  });

  it("extracts session ids for firehose records", () => {
    const record = buildFirehoseRecord({
      id: 10,
      run_id: "run-1",
      frame: 5,
      label: "snapshot",
      created_at: "2025-01-01T00:00:00Z",
      payload: { session_id: "demo_session", action: "move:up" },
    });
    expect(record.session_id).toBe("demo_session");
  });

  it("ignores invalid session ids in payloads", () => {
    const record = buildFirehoseRecord({
      id: 11,
      run_id: "run-2",
      frame: null,
      label: null,
      created_at: "2025-01-02T00:00:00Z",
      payload: { session_id: "bad id" },
    });
    expect(record.session_id).toBeNull();
  });

  it("encodes records as JSONL with a trailing newline", () => {
    const record = buildFirehoseRecord({
      id: 12,
      run_id: "run-3",
      frame: 9,
      label: "snapshot",
      created_at: "2025-01-03T00:00:00Z",
      payload: { session_id: "session-3" },
    });
    const output = encodeFirehoseRecords([record]);
    expect(output.endsWith("\n")).toBe(true);
    const lines = output.trim().split("\n");
    expect(lines).toHaveLength(1);
    expect(JSON.parse(lines[0])).toMatchObject({ id: 12, run_id: "run-3" });
  });
});
