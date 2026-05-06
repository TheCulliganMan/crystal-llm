/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import HomeDesktopPage from "./page";

const mockPlayPanel = jest.fn((_props: Record<string, unknown>) => <div data-testid="shared-play-panel" />);

jest.mock("@/app/play-panel.lazy", () => ({
  __esModule: true,
  default: (props: Record<string, unknown>) => mockPlayPanel(props),
}));

describe("desktop page", () => {
  beforeEach(() => {
    mockPlayPanel.mockClear();
    window.history.replaceState({}, "", "/desktop");
  });

  it("renders the shared KrabbyClaw play panel instead of a separate desktop canvas implementation", () => {
    render(<HomeDesktopPage />);

    const pageShell = screen.getByTestId("desktop-page-shell");
    expect(pageShell).toBeInTheDocument();
    expect(screen.getByTestId("shared-play-panel")).toBeInTheDocument();
    expect(screen.queryByTestId("desktop-agent-panel")).not.toBeInTheDocument();
    expect(mockPlayPanel).toHaveBeenCalledWith({ variant: "desktop" });
  });
});
