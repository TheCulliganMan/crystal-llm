/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { act, fireEvent, render, screen } from "@testing-library/react";

const mockUsePathname = jest.fn();

jest.mock("next/link", () => {
  const LinkMock = ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  );
  LinkMock.displayName = "LinkMock";
  return LinkMock;
});

jest.mock("next/navigation", () => ({
  usePathname: () => mockUsePathname(),
}));

import { MainNav } from "@/components/layout/main-nav";

describe("MainNav", () => {
  beforeEach(() => {
    jest.useFakeTimers();
    mockUsePathname.mockReturnValue("/");
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: jest.fn().mockImplementation(() => ({
        matches: false,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
      })),
    });
  });

  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
    jest.clearAllMocks();
  });

  const hasCurrentPage = (label: string) =>
    screen.getAllByLabelText(label).some((element) => element.getAttribute("aria-current") === "page");

  const getDesktopGameCornerTrigger = () => screen.getByRole("link", { name: "Game Corner" });

  const getGameCornerFlyout = () => screen.getByTestId("game-corner-flyout");

  const getHoverBridge = () => screen.getByTestId("game-corner-hover-bridge");

  it("contains game corner nav item", () => {
    render(<MainNav />);
    expect(screen.getAllByLabelText("Game Corner").length).toBeGreaterThan(0);
  });

  it("shows game corner section links from the dice navbar item", () => {
    render(<MainNav mode="desktop" />);

    expect(screen.getByLabelText("Game Corner: Game Corner")).toHaveAttribute("href", "/game-corner?tab=slot-machine");
    expect(screen.getByLabelText("Game Corner: Arena MCP/Skill")).toHaveAttribute("href", "/game-corner?tab=arena-mcp-skill");
    expect(screen.getByLabelText("Game Corner: Progress Tracker")).toHaveAttribute("href", "/game-corner?tab=progress-tracker");
  });

  it("does not render a separate Game Corner submenu block in desktop nav", () => {
    mockUsePathname.mockReturnValue("/game-corner");
    render(<MainNav mode="desktop" />);

    expect(screen.queryByTestId("game-corner-nav-submenu")).not.toBeInTheDocument();
  });

  it("updates active nav item when pathname changes", () => {
    const { rerender } = render(<MainNav />);
    expect(hasCurrentPage("Play")).toBe(true);
    expect(hasCurrentPage("Leaderboard")).toBe(false);

    mockUsePathname.mockReturnValue("/leaderboard");
    rerender(<MainNav />);

    expect(hasCurrentPage("Leaderboard")).toBe(true);
    expect(hasCurrentPage("Play")).toBe(false);
  });

  it("maps arena routes to leaderboard active state", () => {
    mockUsePathname.mockReturnValue("/arena/live/demo-run");
    render(<MainNav />);

    expect(hasCurrentPage("Leaderboard")).toBe(true);
    expect(hasCurrentPage("Play")).toBe(false);
  });

  it("renders desktop-only links when desktop mode is requested", () => {
    render(<MainNav mode="desktop" />);
    expect(screen.getByRole("navigation", { name: "Main desktop navigation" })).toBeInTheDocument();
    expect(screen.getByLabelText("Game Corner")).toBeInTheDocument();
  });

  it("uses shared gradient surface styling on desktop sidebar", () => {
    const { container } = render(<MainNav mode="desktop" />);
    expect(container.querySelector("aside")).toHaveClass("kc-surface-bar");
  });

  it("renders the home tile full-size and aligned with the sidebar buttons", () => {
    render(<MainNav mode="desktop" />);

    const homeLink = screen.getByLabelText("KrabbyClaw home");
    const playButton = screen.getByLabelText("Play");
    const sprite = homeLink.querySelector(".kc-brand-sprite");

    expect(homeLink).toHaveClass("h-14");
    expect(homeLink).toHaveClass("w-14");
    expect(homeLink).toHaveClass("rounded-lg");
    expect(homeLink).toHaveClass("border");
    expect(homeLink).toHaveClass("border-transparent");
    expect(homeLink).toHaveClass("mx-1");
    expect(homeLink).toHaveClass("overflow-hidden");
    expect(homeLink).toHaveClass("p-0");

    expect(playButton).toHaveClass("h-14");
    expect(playButton).toHaveClass("w-14");
    expect(playButton).toHaveClass("rounded-lg");
    expect(playButton).toHaveClass("border");
    expect(playButton).toHaveClass("border-transparent");

    expect(sprite).toHaveClass("rounded-none");
    expect(sprite).not.toHaveClass("size-5");
    expect(sprite).not.toHaveClass("h-full");
    expect(sprite).not.toHaveClass("w-full");
    expect(sprite).toHaveAttribute("style", expect.stringContaining("transform: scale(1.68)"));
    expect(sprite).toHaveAttribute("style", expect.not.stringContaining("--brand-sprite-size"));
  });

  it("renders the home mascot sprite inside the hover target link", () => {
    render(<MainNav mode="desktop" />);

    const homeLink = screen.getByLabelText("KrabbyClaw home");
    const sprite = homeLink.querySelector(".kc-brand-sprite");

    expect(sprite).not.toBeNull();
    expect(homeLink).toContainElement(sprite);
  });

  it("uses shared gradient surface styling on mobile dock", () => {
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: jest.fn().mockImplementation(() => ({
        matches: true,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
      })),
    });
    const { container } = render(<MainNav mode="mobile" />);
    expect(container.querySelector("nav.dock")).toHaveClass("kc-surface-bar");
    expect(container.querySelector("nav.dock")).toHaveClass("z-[80]");
    expect(container.querySelector("nav.dock")).toHaveClass("overflow-visible");
  });

  it("keeps the desktop game corner flyout above content layers", () => {
    const { container } = render(<MainNav mode="desktop" />);

    const aside = container.querySelector("aside");
    const flyoutMenu = getGameCornerFlyout();

    expect(aside).toHaveClass("z-30");
    expect(flyoutMenu).toHaveClass("z-[90]");
  });

  it("renders the game corner flyout with an opaque panel style", () => {
    render(<MainNav mode="desktop" />);

    const flyoutMenu = getGameCornerFlyout();
    expect(flyoutMenu).toHaveClass("bg-base-100");
    expect(flyoutMenu).toHaveClass("border-base-300");
    expect(flyoutMenu).not.toHaveClass("bg-base-200/95");
  });

  it("opens the game corner flyout on desktop pointer enter", () => {
    render(<MainNav mode="desktop" />);

    const trigger = getDesktopGameCornerTrigger();
    const flyoutMenu = getGameCornerFlyout();

    expect(flyoutMenu).toHaveClass("opacity-0");
    expect(flyoutMenu).toHaveClass("pointer-events-none");

    fireEvent.pointerEnter(trigger);

    expect(flyoutMenu).toHaveClass("opacity-100");
    expect(flyoutMenu).toHaveClass("pointer-events-auto");
  });

  it("keeps the flyout open while crossing the hover bridge into the menu", () => {
    render(<MainNav mode="desktop" />);

    const trigger = getDesktopGameCornerTrigger();
    const bridge = getHoverBridge();
    const flyoutMenu = getGameCornerFlyout();

    fireEvent.pointerEnter(trigger);
    fireEvent.pointerLeave(trigger, { relatedTarget: bridge });
    fireEvent.pointerEnter(bridge);
    fireEvent.pointerLeave(bridge, { relatedTarget: flyoutMenu });
    fireEvent.pointerEnter(flyoutMenu);

    act(() => {
      jest.advanceTimersByTime(220);
    });

    expect(flyoutMenu).toHaveClass("opacity-100");
    expect(flyoutMenu).toHaveClass("pointer-events-auto");
  });

  it("closes the flyout after the close delay when pointer leaves both trigger and menu", () => {
    render(<MainNav mode="desktop" />);

    const trigger = getDesktopGameCornerTrigger();
    const flyoutMenu = getGameCornerFlyout();

    fireEvent.pointerEnter(trigger);
    fireEvent.pointerLeave(trigger, { relatedTarget: document.body });

    act(() => {
      jest.advanceTimersByTime(219);
    });
    expect(flyoutMenu).toHaveClass("opacity-100");

    act(() => {
      jest.advanceTimersByTime(1);
    });
    expect(flyoutMenu).toHaveClass("opacity-0");
    expect(flyoutMenu).toHaveClass("pointer-events-none");
  });

  it("cancels a pending close when the pointer re-enters before the timer fires", () => {
    render(<MainNav mode="desktop" />);

    const trigger = getDesktopGameCornerTrigger();
    const flyoutMenu = getGameCornerFlyout();

    fireEvent.pointerEnter(trigger);
    fireEvent.pointerLeave(trigger, { relatedTarget: document.body });

    act(() => {
      jest.advanceTimersByTime(150);
    });

    fireEvent.pointerEnter(trigger);

    act(() => {
      jest.advanceTimersByTime(220);
    });

    expect(flyoutMenu).toHaveClass("opacity-100");
    expect(flyoutMenu).toHaveClass("pointer-events-auto");
  });

  it("keeps the flyout open while a submenu link has focus and closes after focus leaves", () => {
    render(<MainNav mode="desktop" />);

    const trigger = getDesktopGameCornerTrigger();
    const submenuLink = screen.getByLabelText("Game Corner: Arena MCP/Skill");
    const flyoutMenu = getGameCornerFlyout();

    fireEvent.focus(trigger);
    expect(flyoutMenu).toHaveClass("opacity-100");

    fireEvent.blur(trigger, { relatedTarget: submenuLink });
    fireEvent.focus(submenuLink);

    act(() => {
      jest.advanceTimersByTime(220);
    });
    expect(flyoutMenu).toHaveClass("opacity-100");

    fireEvent.blur(submenuLink, { relatedTarget: document.body });

    act(() => {
      jest.advanceTimersByTime(220);
    });
    expect(flyoutMenu).toHaveClass("opacity-0");
  });

  it("keeps mobile game corner navigation pointing at the default slot machine tab", () => {
    render(<MainNav mode="mobile" />);

    expect(screen.getByRole("link", { name: "Game Corner" })).toHaveAttribute(
      "href",
      "/game-corner?tab=slot-machine"
    );
  });
});
