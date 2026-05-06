/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import type { ArenaRun } from "@/arena/types";

const mockResolveWatchRuns = jest.fn();

jest.mock("@/components/arena/watch-runs", () => ({
  WatchRunList: ({ limit, initialRuns }: { limit?: number; initialRuns?: ArenaRun[] }) => (
    <div data-testid="watch-run-list" data-limit={limit ?? 0} data-initial-count={initialRuns?.length ?? 0}>
      Watch Run List
    </div>
  ),
}));

jest.mock("@/arena/watch-resolver", () => ({
  MAX_WATCH_SESSION_LIMIT: 27,
  resolveWatchRuns: (...args: unknown[]) => mockResolveWatchRuns(...args),
}));

import WatchPage from "@/app/watch/page";

describe("WatchPage", () => {
  beforeEach(() => {
    mockResolveWatchRuns.mockResolvedValue({
      ok: true,
      runs: [{ id: "run-1" }],
    });
  });

  it("renders watch heading and breadth-focused helper copy", async () => {
    render(await WatchPage());

    expect(screen.getByTestId("route-watch")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Watch" })).toBeInTheDocument();
    expect(screen.getByText(/coverage across agent types and queue lanes/i)).toBeInTheDocument();
    expect(screen.getByTestId("watch-run-list")).toHaveAttribute("data-limit", "27");
    expect(screen.getByTestId("watch-run-list")).toHaveAttribute("data-initial-count", "1");
  });
});
