/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import HomePage from "./page";

jest.mock("./play-panel.lazy", () => ({
  __esModule: true,
  default: () => <div data-testid="play-panel">Play panel</div>,
}));

describe("HomePage", () => {
  it("renders the play panel as a viewport shell", () => {
    render(<HomePage />);

    const pageShell = screen.getByTestId("play-page-shell");
    expect(pageShell).toBeInTheDocument();
    expect(pageShell).toHaveClass(
      "overflow-hidden",
      "h-[calc(100dvh-var(--nav-top-height)-var(--nav-mobile-height)-0.35rem)]",
      "md:h-[calc(100dvh-var(--nav-top-height)-1rem)]"
    );
    expect(screen.getByTestId("play-panel")).toBeInTheDocument();
  });
});
