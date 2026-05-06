/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { act, render, screen, waitFor } from "@testing-library/react";
import { KrabbyClawArenaPanel } from "@/app/game-corner/krabbyclaw-arena-panel";

const fetchMock = jest.fn();

describe("KrabbyClawArenaPanel", () => {
  beforeEach(() => {
    fetchMock.mockReset();
    global.fetch = fetchMock as unknown as typeof fetch;
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({
        ok: true,
        leaderboard: [],
        activeMatches: [],
        recentMatches: [],
        agents: {},
      }),
    });
  });

  it("renders arena setup controls and downloads", async () => {
    await act(async () => {
      render(<KrabbyClawArenaPanel />);
    });

    expect(screen.getByText("KrabbyClaw Colosseum")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Queue challenger" })).toBeInTheDocument();

    const download = screen.getByRole("link", { name: "Download Arena Skill" });
    expect(download).toHaveAttribute("href", "/downloads/krabbyclaw-arena-skill.zip");

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith("/api/arena/krabbyclaw?limit=16", expect.any(Object));
    });
  });

  it("renders leaderboard rows from API snapshot", async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        ok: true,
        leaderboard: [
          {
            rank: 1,
            agent_id: "agent-1",
            agent_name: "Krabby Prime",
            rating: 1042,
            games_played: 12,
            wins: 7,
            losses: 4,
            draws: 1,
            win_rate: 58.3,
          },
        ],
        activeMatches: [],
        recentMatches: [],
        agents: {},
      }),
    });

    await act(async () => {
      render(<KrabbyClawArenaPanel />);
    });

    expect(await screen.findByText("Krabby Prime")).toBeInTheDocument();
    expect(screen.getByText("1042")).toBeInTheDocument();
  });

  it("requests advancing battle frames for live feed tiles", async () => {
    jest.useFakeTimers();
    fetchMock.mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.startsWith("/api/arena/frame?")) {
        return {
          ok: true,
          json: async () => ({ ok: true, image: "ZmFrZQ==" }),
        };
      }
      return {
        ok: true,
        json: async () => ({
          ok: true,
          leaderboard: [],
          queue: [],
          activeMatches: [
            {
              id: "match-1",
              challenger_agent_id: "agent-1",
              opponent_agent_id: "agent-2",
              status: "running",
              outcome: null,
              winner_agent_id: null,
              queue: "krabbyclaw-arena",
              challenger_session_id: "krabby-prime",
              opponent_session_id: "kingler-core",
              challenger_score: null,
              opponent_score: null,
              started_at: null,
              finished_at: null,
              created_at: new Date().toISOString(),
            },
          ],
          recentMatches: [],
          agents: {
            "agent-1": { id: "agent-1", name: "Krabby Prime", slug: "krabby-prime", runtime: "mcp-http" },
            "agent-2": { id: "agent-2", name: "Kingler Core", slug: "kingler-core", runtime: "mcp-http" },
          },
        }),
      };
    });

    await act(async () => {
      render(<KrabbyClawArenaPanel />);
    });

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/arena/frame?session_id=krabby-prime&scale=2&advance=24"),
        expect.any(Object)
      );
    });

    jest.useRealTimers();
  });

});
