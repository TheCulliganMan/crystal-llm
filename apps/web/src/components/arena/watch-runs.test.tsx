/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import { WatchRunList } from "./watch-runs";
import type { ArenaAgentRow, ArenaRun } from "@/arena/types";

const mockCreateSupabaseBrowserClient = jest.fn();
const mockIsSupabaseConfigured = jest.fn();

jest.mock("@/lib/supabase/browser", () => ({
  createSupabaseBrowserClient: () => mockCreateSupabaseBrowserClient(),
}));

jest.mock("@/lib/supabase/env", () => ({
  isSupabaseConfigured: () => mockIsSupabaseConfigured(),
}));

const mockGameCanvas = jest.fn((props: Record<string, unknown>) => (
  <div
    data-testid="watch-game-canvas"
    data-session-id={String(props.sessionId ?? "")}
    data-remote-visual-mode={String(props.remoteVisualMode ?? "")}
    data-read-only={String(props.readOnly ?? false)}
    data-remote-frame-scale={String(props.remoteFrameScale ?? "")}
    data-remote-advance-frames={String(props.remoteAdvanceFrames ?? "")}
    data-remote-frame-refresh-key={String(props.remoteFrameRefreshKey ?? "")}
  />
));

jest.mock("@/app/game-canvas", () => ({
  GameCanvas: (props: Record<string, unknown>) => mockGameCanvas(props),
}));

type TestGlobalThis = {
  fetch?: typeof globalThis.fetch;
};

const testGlobal = globalThis as TestGlobalThis;

const buildAgent = (overrides: Partial<ArenaAgentRow> = {}): ArenaAgentRow => ({
  id: "agent-1",
  owner_id: "user-1",
  name: "Real Agent",
  slug: "real-agent",
  description: null,
  repo_url: null,
  mcp_endpoint: null,
  runtime: "mcp-http",
  visibility: "public",
  config: {},
  created_at: "2025-01-01T00:00:00Z",
  updated_at: "2025-01-01T00:00:00Z",
  ...overrides,
});

const buildRun = (overrides: Partial<ArenaRun> = {}): ArenaRun => ({
  id: "run-1",
  agent_id: "agent-1",
  created_by: "user-1",
  status: "running",
  queue: "arena",
  seed: null,
  mcp_session_url: null,
  spectator_frame_url: null,
  started_at: null,
  finished_at: null,
  frame_count: 120,
  badge_count: 0,
  pokedex_seen: 0,
  pokedex_caught: 0,
  error: null,
  metrics: {
    last_snapshot_text: "INFO:\nPlayer's Room - pacing by the PC.",
    session_id: "11111111-1111-4111-8111-111111111111",
  },
  notes: null,
  created_at: "2025-01-01T00:00:00Z",
  updated_at: "2025-01-01T00:00:00Z",
  agent: buildAgent(),
  ...overrides,
});

const renderWatchRunList = async (
  props: { initialRuns: ArenaRun[]; limit?: number }
): Promise<void> => {
  await act(async () => {
    render(<WatchRunList {...props} />);
    await Promise.resolve();
    await Promise.resolve();
  });
};

type MockChannel = {
  name: string;
  on: jest.Mock;
  subscribe: jest.Mock;
  handlers: Array<{ type: string; filter: Record<string, unknown>; callback: (payload: unknown) => void }>;
  statusCallback?: (status: string) => void;
};

const buildRealtimeClient = () => {
  const channels: MockChannel[] = [];
  const client = {
    channel: jest.fn((name: string) => {
      const channel: MockChannel = {
        name,
        handlers: [],
        on: jest.fn((type: string, filter: Record<string, unknown>, callback: (payload: unknown) => void) => {
          channel.handlers.push({ type, filter, callback });
          return channel;
        }),
        subscribe: jest.fn((callback?: (status: string) => void) => {
          channel.statusCallback = callback;
          return channel;
        }),
      };
      channels.push(channel);
      return channel;
    }),
    removeChannel: jest.fn(),
  };
  return { client, channels };
};

describe("WatchRunList", () => {
  const originalFetch = testGlobal.fetch;
  const originalSetInterval = globalThis.setInterval;
  const originalClearInterval = globalThis.clearInterval;
  let mockedRunsResponses: Array<{ ok: boolean; status?: number; runs?: ArenaRun[]; error?: string }> = [];
  let intervalCallbacks: Array<() => void | Promise<void>> = [];

  const pollIntervals = async (): Promise<void> => {
    for (const callback of [...intervalCallbacks]) {
      await act(async () => {
        await callback();
        await Promise.resolve();
        await Promise.resolve();
      });
    }
  };

  beforeEach(() => {
    mockedRunsResponses = [];
    intervalCallbacks = [];
    mockCreateSupabaseBrowserClient.mockReset();
    mockIsSupabaseConfigured.mockReset();
    mockCreateSupabaseBrowserClient.mockReturnValue(null);
    mockIsSupabaseConfigured.mockReturnValue(false);
    mockGameCanvas.mockClear();
    testGlobal.fetch = jest.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/arena/runs")) {
        const payload = mockedRunsResponses.shift() ?? { ok: true, runs: [] as ArenaRun[] };
        return {
          ok: payload.ok,
          status: payload.status ?? (payload.ok ? 200 : 500),
          json: async () => payload,
        } as Response;
      }
      return {
        ok: true,
        status: 200,
        json: async () => ({
          ok: true,
          image: "dGVzdA==",
          width: 160,
          height: 144,
          frame: 1,
        }),
      } as Response;
    }) as unknown as typeof globalThis.fetch;
    globalThis.setInterval = jest.fn((callback: TimerHandler) => {
      if (typeof callback === "function") {
        intervalCallbacks.push(callback as () => void | Promise<void>);
      }
      return intervalCallbacks.length as unknown as ReturnType<typeof setInterval>;
    });
    globalThis.clearInterval = jest.fn();
  });

  afterEach(() => {
    testGlobal.fetch = originalFetch;
    globalThis.setInterval = originalSetInterval;
    globalThis.clearInterval = originalClearInterval;
  });

  it("shows a real empty state instead of simulated fallback feeds", async () => {
    mockedRunsResponses = [{ ok: true, runs: [] }];

    await renderWatchRunList({ initialRuns: [], limit: 24 });

    expect(await screen.findByText("No live sessions right now")).toBeInTheDocument();
    expect(screen.getByText(/watch real Pokemon Crystal gameplay here/i)).toBeInTheDocument();
    expect(screen.queryByText("Amber Atlas")).not.toBeInTheDocument();
  });

  it("renders real runs and keeps frame requests read-only", async () => {
    const run = buildRun({ id: "run-99" });
    mockedRunsResponses = [{ ok: true, runs: [run] }];

    await renderWatchRunList({ initialRuns: [run], limit: 1 });

    expect(screen.getByText("Real Agent")).toBeInTheDocument();
    expect(await screen.findByLabelText("Real Agent live feed")).toBeInTheDocument();
    expect(mockGameCanvas).toHaveBeenCalledWith(
      expect.objectContaining({
        sessionId: "11111111-1111-4111-8111-111111111111",
        runtimeMode: "server",
        remoteVisualMode: "frame",
        readOnly: true,
        remoteFrameScale: 2,
        remoteAdvanceFrames: 0,
      })
    );
  });

  it("opens fullscreen details when a run is clicked", async () => {
    const run = buildRun({ id: "run-42", agent: buildAgent({ name: "Click Agent" }) });
    mockedRunsResponses = [{ ok: true, runs: [run] }];

    await renderWatchRunList({ initialRuns: [run], limit: 1 });

    await screen.findByLabelText("Click Agent live feed");
    fireEvent.click(screen.getByLabelText("Open Click Agent run"));

    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(await screen.findByLabelText("Click Agent fullscreen feed")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close viewer" })).toBeInTheDocument();
    expect(screen.getByText("Badges: 0")).toBeInTheDocument();
    expect(screen.getByText("Pokedex: 0 seen")).toBeInTheDocument();
    expect(screen.getByText("Caught: 0")).toBeInTheDocument();
    expect(screen.getByText("Agent links")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Open live console" })).toHaveAttribute("href", "/arena/live/run-42");
    expect(mockGameCanvas).toHaveBeenCalledWith(expect.objectContaining({ remoteFrameScale: 4 }));
  });

  it("uses friendly runtime labels and stable names when agent names are missing", async () => {
    const run = buildRun({
      agent: buildAgent({ name: "", runtime: "mcp-http" }),
      metrics: {
        last_snapshot_text: "INFO:\nPlayer's Room - pacing by the PC.",
        session_id: "00000000-0000-4000-8000-000000000000",
      },
    });
    mockedRunsResponses = [{ ok: true, runs: [run] }];

    await renderWatchRunList({ initialRuns: [run], limit: 1 });

    expect(screen.getByText("Session 00000000")).toBeInTheDocument();
    expect(screen.getAllByText("Arena Live")).not.toHaveLength(0);
  });

  it("opens the correct run details when multiple runs are listed", async () => {
    const firstRun = buildRun({
      id: "run-1",
      agent: buildAgent({ name: "First Agent" }),
      metrics: { session_id: "session-first" },
    });
    const secondRun = buildRun({
      id: "run-2",
      agent: buildAgent({ name: "Second Agent" }),
      metrics: { session_id: "session-second" },
    });
    mockedRunsResponses = [{ ok: true, runs: [firstRun, secondRun] }];

    await renderWatchRunList({ initialRuns: [firstRun, secondRun], limit: 24 });

    await screen.findByLabelText("First Agent live feed");
    fireEvent.click(screen.getByLabelText("Open Second Agent run"));

    const dialog = await screen.findByRole("dialog");
    expect(dialog).toBeInTheDocument();
    const heading = within(dialog).getByRole("heading", { name: /Second Agent/, level: 2 });
    expect(heading).toBeInTheDocument();
    expect(heading).toHaveTextContent("running");
  });

  it("requests up to 27 sessions and renders all available live tiles without pagination", async () => {
    const realRuns: ArenaRun[] = Array.from({ length: 12 }, (_, i) =>
      buildRun({
        id: `run-${i + 1}`,
        agent: buildAgent({ id: `agent-${i + 1}`, name: `Agent ${i + 1}` }),
        created_by: "agent-2",
        metrics: { session_id: `session-${i + 1}` },
      })
    );
    mockedRunsResponses = [{ ok: true, runs: realRuns }];

    await renderWatchRunList({ initialRuns: [], limit: 24 });

    const fetchMock = testGlobal.fetch as jest.Mock;
    await screen.findByLabelText("Agent 1 live feed");
    const runsRequest = fetchMock.mock.calls.find((call) => String(call[0]).includes("/api/arena/runs"));
    const callUrl = runsRequest ? new URL(String(runsRequest[0]), "http://localhost") : null;
    if (!callUrl) {
      throw new Error("arena runs request was not made");
    }
    expect(callUrl.searchParams.get("limit")).toBe("24");

    const visibleFrames = await screen.findAllByLabelText(/Agent \d+ live feed/);
    expect(visibleFrames).toHaveLength(12);
    expect(screen.queryByRole("tab", { name: "2" })).not.toBeInTheDocument();
  });

  it("keeps showing the last successful live runs when a poll fails", async () => {
    const run = buildRun({ id: "run-stable", agent: buildAgent({ name: "Stable Agent" }) });
    mockedRunsResponses = [
      { ok: true, runs: [run] },
      { ok: false, status: 500, error: "still broken" },
    ];

    await renderWatchRunList({ initialRuns: [run], limit: 24 });
    expect(await screen.findByText("Stable Agent")).toBeInTheDocument();

    await pollIntervals();

    expect(screen.getByText("Stable Agent")).toBeInTheDocument();
    expect(screen.queryByText("No live sessions right now")).not.toBeInTheDocument();
  });

  it("gives runs two missed polls before removing them from the wall", async () => {
    const run = buildRun({ id: "run-grace", agent: buildAgent({ name: "Grace Agent" }) });
    mockedRunsResponses = [
      { ok: true, runs: [run] },
      { ok: true, runs: [] },
      { ok: true, runs: [] },
      { ok: true, runs: [] },
    ];

    await renderWatchRunList({ initialRuns: [run], limit: 24 });
    expect(await screen.findByText("Grace Agent")).toBeInTheDocument();

    await pollIntervals();
    expect(screen.getByText("Grace Agent")).toBeInTheDocument();

    await pollIntervals();
    expect(screen.getByText("Grace Agent")).toBeInTheDocument();

    await pollIntervals();
    await waitFor(() => expect(screen.queryByText("Grace Agent")).not.toBeInTheDocument());
    expect(screen.getByText("No live sessions right now")).toBeInTheDocument();
  });

  it("deduplicates multiple real runs that resolve to the same session id", async () => {
    const firstRun = buildRun({
      id: "first-same-session",
      agent: buildAgent({ name: "First Real Session Agent" }),
      metrics: { session_id: "same-session" },
      updated_at: "2025-01-01T00:00:00Z",
    });
    const secondRun = buildRun({
      id: "second-same-session",
      agent: buildAgent({ name: "Second Real Session Agent" }),
      metrics: { session_id: "same-session" },
      updated_at: "2025-01-01T00:01:00Z",
    });

    mockedRunsResponses = [{ ok: true, runs: [firstRun, secondRun] }];

    await renderWatchRunList({ initialRuns: [firstRun, secondRun], limit: 10 });

    expect(await screen.findByText("First Real Session Agent")).toBeInTheDocument();
    expect(screen.queryByText("Second Real Session Agent")).not.toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /^Open / })).toHaveLength(1);
  });

  it("merges realtime run inserts and updates without waiting for polling", async () => {
    const realtime = buildRealtimeClient();
    mockCreateSupabaseBrowserClient.mockReturnValue(realtime.client);
    mockIsSupabaseConfigured.mockReturnValue(true);
    mockedRunsResponses = [{ ok: true, runs: [] }];

    await renderWatchRunList({ initialRuns: [], limit: 24 });

    const runsChannel = realtime.channels.find((channel) => channel.name === "arena-runs-watch");
    await act(async () => {
      runsChannel?.statusCallback?.("SUBSCRIBED");
      await Promise.resolve();
    });
    const changeHandler = runsChannel?.handlers.find((handler) => handler.type === "postgres_changes")?.callback;
    if (!changeHandler) {
      throw new Error("missing realtime run handler");
    }

    await act(async () => {
      changeHandler({
        eventType: "INSERT",
        new: buildRun({
          id: "realtime-run",
          agent: undefined,
          metrics: { session_id: "realtime-session" },
          frame_count: 10,
        }),
      });
      await Promise.resolve();
    });

    expect(await screen.findByText("Session realtime")).toBeInTheDocument();
    expect(screen.getByText("Realtime")).toBeInTheDocument();

    await act(async () => {
      changeHandler({
        eventType: "UPDATE",
        new: {
          id: "realtime-run",
          status: "running",
          frame_count: 99,
          metrics: { session_id: "realtime-session" },
        },
      });
      await Promise.resolve();
    });

    fireEvent.click(screen.getByLabelText("Open Session realtime run"));
    expect(await screen.findByText("Frames: 99")).toBeInTheDocument();
  });

  it("subscribes to session snapshots and triggers immediate frame refreshes", async () => {
    const realtime = buildRealtimeClient();
    mockCreateSupabaseBrowserClient.mockReturnValue(realtime.client);
    mockIsSupabaseConfigured.mockReturnValue(true);
    const run = buildRun({
      id: "snapshot-run",
      agent: buildAgent({ name: "Snapshot Agent" }),
      metrics: { session_id: "snapshot-session" },
    });
    mockedRunsResponses = [{ ok: true, runs: [run] }];

    await renderWatchRunList({ initialRuns: [run], limit: 24 });
    expect(await screen.findByLabelText("Snapshot Agent live feed")).toBeInTheDocument();

    const sessionChannel = await waitFor(() => {
      const found = [...realtime.channels].reverse().find((channel) => channel.name === "arena-session:snapshot-session");
      expect(found).toBeDefined();
      return found as MockChannel;
    });
    const snapshotHandler = sessionChannel.handlers.find((handler) => handler.type === "broadcast")?.callback;
    if (!snapshotHandler) {
      throw new Error("missing snapshot handler");
    }

    const firstCanvas = screen.getAllByTestId("watch-game-canvas")[0];
    expect(firstCanvas).toHaveAttribute("data-remote-frame-refresh-key", "0");

    await act(async () => {
      snapshotHandler({
        payload: {
          run_id: "snapshot-run",
          session_id: "snapshot-session",
          text: "OVERWORLD\nLatest live snapshot",
          action: "move:right",
          frame: 321,
        },
      });
      await Promise.resolve();
    });

    await waitFor(() =>
      expect(screen.getAllByTestId("watch-game-canvas")[0]).toHaveAttribute("data-remote-frame-refresh-key", "1")
    );
    fireEvent.click(screen.getByLabelText("Open Snapshot Agent run"));
    expect(await screen.findByText("Latest action")).toBeInTheDocument();
    expect(screen.getByText("move:right")).toBeInTheDocument();
    expect(screen.getByText(/Latest live snapshot/)).toBeInTheDocument();
    expect(screen.getByText("Frames: 321")).toBeInTheDocument();
  });

  it("removes a run when realtime reports a terminal status", async () => {
    const realtime = buildRealtimeClient();
    mockCreateSupabaseBrowserClient.mockReturnValue(realtime.client);
    mockIsSupabaseConfigured.mockReturnValue(true);
    const run = buildRun({ id: "terminal-run", agent: buildAgent({ name: "Terminal Agent" }) });
    mockedRunsResponses = [{ ok: true, runs: [run] }];

    await renderWatchRunList({ initialRuns: [run], limit: 24 });
    expect(await screen.findByText("Terminal Agent")).toBeInTheDocument();

    const runsChannel = realtime.channels.find((channel) => channel.name === "arena-runs-watch");
    const changeHandler = runsChannel?.handlers.find((handler) => handler.type === "postgres_changes")?.callback;
    if (!changeHandler) {
      throw new Error("missing realtime run handler");
    }

    await act(async () => {
      changeHandler({
        eventType: "UPDATE",
        new: { id: "terminal-run", status: "completed" },
      });
      await Promise.resolve();
    });

    await waitFor(() => expect(screen.queryByText("Terminal Agent")).not.toBeInTheDocument());
    expect(screen.getByText("No live sessions right now")).toBeInTheDocument();
  });
});
