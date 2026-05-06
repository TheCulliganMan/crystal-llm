/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import { GameCornerPageContent } from "@/app/game-corner/page-content";

jest.mock("@/app/game-corner/game-corner-shell", () => ({
  GameCornerShell: ({ initialTab }: { initialTab?: string }) => (
    <div data-testid="game-corner-shell" data-initial-tab={initialTab ?? "slot-machine"}>
      Game Corner Shell
    </div>
  ),
}));

describe("GameCornerPageContent", () => {
  it("renders a stable route shell marker for Playwright harnesses", () => {
    render(<GameCornerPageContent initialTab="arena-mcp-skill" />);

    expect(screen.getByTestId("route-game-corner")).toBeInTheDocument();
    expect(screen.getByTestId("game-corner-shell")).toHaveAttribute("data-initial-tab", "arena-mcp-skill");
  });
});
