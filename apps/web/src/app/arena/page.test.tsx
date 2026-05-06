/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import ArenaPage from "./page";

jest.mock("@/arena/queries", () => ({
  fetchLeaderboard: jest.fn(async () => ({ leaderboard: [], agents: [] })),
  fetchActiveRuns: jest.fn(async () => []),
  fetchRecentRuns: jest.fn(async () => []),
}));

jest.mock("@/arena/progress", () => ({
  buildLatestProgressRows: jest.fn(() => []),
}));

describe("ArenaPage", () => {
  it("always uses compact layout without exposing a density toggle", async () => {
    render(await ArenaPage());

    expect(screen.getByTestId("route-arena")).toBeInTheDocument();
    expect(screen.getByText("KrabbyClaw")).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Compact density" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Standard density" })).not.toBeInTheDocument();
    expect(document.querySelector('a[href*="density=compact"]')).toBeNull();
  });
});
