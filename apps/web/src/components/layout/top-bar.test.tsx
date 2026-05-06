/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";

const mockUseSupabase = jest.fn();
const mockUsePathname = jest.fn();

jest.mock("@/components/providers/supabase-provider", () => ({
  useSupabase: () => mockUseSupabase(),
}));

jest.mock("@/components/arena/auth-panel", () => ({
  AuthPanel: () => <div>Auth Panel</div>,
}));

jest.mock("next/navigation", () => ({
  usePathname: () => mockUsePathname(),
}));

import { TopBar } from "./top-bar";

describe("TopBar", () => {
  beforeEach(() => {
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: jest.fn().mockImplementation((query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
        addListener: jest.fn(),
        removeListener: jest.fn(),
        dispatchEvent: jest.fn(),
      })),
    });
    mockUsePathname.mockReturnValue("/");
    mockUseSupabase.mockReturnValue({
      supabaseClient: null,
      session: null,
      isConfigured: false,
    });
  });

  it("hides login controls on the rank page", () => {
    mockUsePathname.mockReturnValue("/leaderboard");
    render(<TopBar />);

    expect(screen.getByRole("button", { name: /login/i })).toBeInTheDocument();
    expect(screen.getByText("Leaderboard")).toBeInTheDocument();
  });

  it("shows login controls on non-rank pages", () => {
    mockUsePathname.mockReturnValue("/");
    render(<TopBar />);

    expect(screen.getByRole("button", { name: /login/i })).toBeInTheDocument();
    expect(screen.getByText("Play")).toBeInTheDocument();
  });

  it("renders route label and app brand", () => {
    mockUsePathname.mockReturnValue("/watch");
    render(<TopBar />);

    expect(screen.getByText("Watch")).toBeInTheDocument();
    expect(screen.getByText("KrabbyClaw")).toBeInTheDocument();
  });

  it("renders game corner route label", () => {
    mockUsePathname.mockReturnValue("/game-corner");
    render(<TopBar />);

    expect(screen.getByText("Game Corner")).toBeInTheDocument();
  });

  it("uses shared gradient surface styling on the top bar", () => {
    render(<TopBar />);
    expect(screen.getByRole("navigation")).toHaveClass("kc-surface-bar");
  });



  it("falls back to play label on legacy audio route", () => {
    mockUsePathname.mockReturnValue("/audio");
    render(<TopBar />);

    expect(screen.getByText("Play")).toBeInTheDocument();
  });
  it("updates route label and auth controls when pathname changes", () => {
    const { rerender } = render(<TopBar />);
    expect(screen.getByText("Play")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /login/i })).toBeInTheDocument();

    mockUsePathname.mockReturnValue("/leaderboard");
    rerender(<TopBar />);

    expect(screen.getByText("Leaderboard")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /login/i })).toBeInTheDocument();
  });
});
