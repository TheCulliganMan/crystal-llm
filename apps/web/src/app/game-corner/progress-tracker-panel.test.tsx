/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

jest.mock("mermaid", () => ({
  __esModule: true,
  default: {
    initialize: jest.fn(),
    render: jest.fn(async (_id: string, graphText: string) => ({
      svg: `<svg data-graph="${graphText.includes('red-defeated') ? 'ok' : 'bad'}"></svg>`,
    })),
  },
}), { virtual: true });

import { ProgressTrackerPanel } from "@/app/game-corner/progress-tracker-panel";

describe("ProgressTrackerPanel", () => {
  it("only allows reachable steps to be checked", async () => {
    render(<ProgressTrackerPanel />);

    expect(
      screen.getByRole("link", { name: "Download Progress Tracker Skill" }),
    ).toHaveAttribute("href", "/downloads/krabbyclaw-progress-tracker-skill.zip");

    const starter = screen.getByLabelText("Mark Starter + Pokédex complete") as HTMLInputElement;
    const red = screen.getByLabelText("Mark Defeat Red complete") as HTMLInputElement;

    expect(starter.disabled).toBe(false);
    expect(red.disabled).toBe(true);

    fireEvent.click(starter);

    await waitFor(() => {
      expect(starter.checked).toBe(true);
    });

    const mrPokemon = screen.getByLabelText("Mark Mr. Pokémon + Mystery Egg complete") as HTMLInputElement;
    expect(mrPokemon.disabled).toBe(false);
  });
});
