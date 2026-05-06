/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import { RouteShell } from "./route-shell";

const mockUsePathname = jest.fn();

jest.mock("next/navigation", () => ({
  usePathname: () => mockUsePathname(),
}));

jest.mock("@/components/layout/main-nav", () => ({
  MainNav: ({ mode }: { mode: string }) => <div data-testid={`main-nav-${mode}`} />,
}));

jest.mock("@/components/layout/top-bar", () => ({
  TopBar: () => <div data-testid="top-bar" />,
}));

describe("RouteShell", () => {
  beforeEach(() => {
    mockUsePathname.mockReset();
  });

  it("renders full desktop UI chrome for non-desktop routes", () => {
    mockUsePathname.mockReturnValue("/");

    render(
      <RouteShell>
        <div data-testid="route-content" />
      </RouteShell>,
    );

    expect(screen.getByTestId("main-nav-desktop")).toBeInTheDocument();
    expect(screen.getByTestId("main-nav-mobile")).toBeInTheDocument();
    expect(screen.getByTestId("top-bar")).toBeInTheDocument();
    expect(screen.getByTestId("route-content")).toBeInTheDocument();
  });

  it("renders only a minimal shell for the /desktop route", () => {
    mockUsePathname.mockReturnValue("/desktop");

    render(
      <RouteShell>
        <div data-testid="route-content" />
      </RouteShell>,
    );

    expect(screen.queryByTestId("main-nav-desktop")).not.toBeInTheDocument();
    expect(screen.queryByTestId("main-nav-mobile")).not.toBeInTheDocument();
    expect(screen.queryByTestId("top-bar")).not.toBeInTheDocument();
    expect(screen.getByTestId("route-content")).toBeInTheDocument();
  });
});
