/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import { GameCornerShell } from "@/app/game-corner/game-corner-shell";

jest.mock("@/app/game-corner/game-corner-client", () => ({
  GameCornerClient: ({ initialTab }: { initialTab?: string }) => (
    <div data-testid="slot-machine-client" data-initial-tab={initialTab ?? "slot-machine"}>
      Slot Machine Client
    </div>
  ),
}));

jest.mock("@/app/game-corner/krabbyclaw-arena-panel", () => ({
  KrabbyClawArenaPanel: () => <div data-testid="krabbyclaw-arena-panel">KrabbyClaw Arena Panel</div>,
}));

jest.mock("@/app/game-corner/progress-tracker-panel", () => ({
  ProgressTrackerPanel: () => <div data-testid="progress-tracker-panel">Progress Tracker Panel</div>,
}));

describe("GameCornerShell", () => {
  it("renders game-corner content by default", () => {
    render(<GameCornerShell />);

    expect(screen.getByText("Goldenrod Game Corner")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Game Corner" })).toBeInTheDocument();
    expect(screen.getByTestId("game-corner-slot-machine")).toBeInTheDocument();
    expect(screen.getByTestId("slot-machine-client")).toHaveAttribute("data-initial-tab", "slot-machine");
  });
  it("renders arena mcp/skill content and quick links", () => {
    render(<GameCornerShell initialTab="arena-mcp-skill" />);

    expect(screen.getByRole("heading", { name: "Arena MCP/Skill" })).toBeInTheDocument();
    expect(screen.getByTestId("game-corner-arena-mcp-skill")).toBeInTheDocument();
    expect(screen.getByTestId("krabbyclaw-arena-panel")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Open MCP Console" })).toHaveAttribute("href", "/mcp");
    expect(screen.getByRole("link", { name: "Download Progress Tracker Skill" }))
      .toHaveAttribute("href", "/downloads/krabbyclaw-progress-tracker-skill.zip");
  });

  it("renders progress tracker content", () => {
    render(<GameCornerShell initialTab="progress-tracker" />);

    expect(screen.getByRole("heading", { name: "Progress Tracker" })).toBeInTheDocument();
    expect(screen.getByTestId("game-corner-progress-tracker")).toBeInTheDocument();
    expect(screen.getByTestId("progress-tracker-panel")).toBeInTheDocument();
  });
});
