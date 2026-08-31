/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";

jest.mock("@/app/game-corner/game-corner-shell", () => ({
  GameCornerShell: ({ initialTab }: { initialTab?: string }) => (
    <div data-testid="game-corner-shell" data-initial-tab={initialTab ?? "slot-machine"}>
      Game Corner Shell
    </div>
  ),
}));

import GameCornerPage from "@/app/game-corner/page";

describe("GameCornerPage", () => {
  it("renders the game corner shell with slot-machine default", async () => {
    render(await GameCornerPage({}));

    const shell = screen.getByTestId("game-corner-shell");
    expect(shell).toHaveAttribute("data-initial-tab", "slot-machine");
  });

  it("accepts arena-mcp-skill tab from query params", async () => {
    render(await GameCornerPage({ searchParams: Promise.resolve({ tab: "arena-mcp-skill" }) }));

    const shell = screen.getByTestId("game-corner-shell");
    expect(shell).toHaveAttribute("data-initial-tab", "arena-mcp-skill");
  });


  it("accepts progress-tracker tab from query params", async () => {
    render(await GameCornerPage({ searchParams: Promise.resolve({ tab: "progress-tracker" }) }));

    const shell = screen.getByTestId("game-corner-shell");
    expect(shell).toHaveAttribute("data-initial-tab", "progress-tracker");
  });

  it("ignores unsupported query tabs and falls back to slot-machine", async () => {
    render(await GameCornerPage({ searchParams: Promise.resolve({ tab: "unknown" }) }));

    const shell = screen.getByTestId("game-corner-shell");
    expect(shell).toHaveAttribute("data-initial-tab", "slot-machine");
  });
});
