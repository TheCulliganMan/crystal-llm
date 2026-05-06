/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { GameCornerClient } from "@/app/game-corner/game-corner-client";

const readCoinCount = (): number => {
  const text = screen.getByTestId("game-corner-coins").textContent ?? "";
  const [, rawValue] = text.split(":");
  return Number(rawValue?.trim() ?? "0");
};

const readState = () => screen.getByTestId("game-corner-state");

describe("GameCornerClient", () => {
  it("renders the slot-machine-only layout", () => {
    render(<GameCornerClient />);

    expect(screen.getByTestId("slot-machine-only-layout")).toBeInTheDocument();
    expect(screen.getByTestId("game-corner-canvas-shell")).toBeInTheDocument();
    expect(screen.getByTestId("game-corner-canvas-frame")).toBeInTheDocument();
    expect(screen.getByTestId("game-corner-tile-canvas")).toBeInTheDocument();
    expect(screen.queryByRole("tab")).not.toBeInTheDocument();
  });

  it("keeps the tile canvas at a fixed gameboy aspect ratio in layout", async () => {
    render(<GameCornerClient />);

    const canvas = screen.getByTestId("game-corner-tile-canvas") as HTMLCanvasElement;
    await waitFor(() => {
      expect(canvas.style.width).toMatch(/px$/);
      expect(canvas.style.height).toMatch(/px$/);
    });
    expect(canvas.style.imageRendering).toBe("pixelated");
    expect(canvas).toHaveAttribute("width", "320");
    expect(canvas).toHaveAttribute("height", "288");

    const displayWidth = Number.parseInt(canvas.style.width, 10);
    const displayHeight = Number.parseInt(canvas.style.height, 10);
    expect(displayWidth).toBeGreaterThan(0);
    expect(displayHeight).toBeGreaterThan(0);
    expect(displayWidth / displayHeight).toBeCloseTo(320 / 288, 2);
  });

  it("resizes the tile canvas display dimensions with viewport changes", async () => {
    const originalInnerWidth = window.innerWidth;
    const originalInnerHeight = window.innerHeight;
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 1200 });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: 900 });

    render(<GameCornerClient />);

    const canvas = screen.getByTestId("game-corner-tile-canvas") as HTMLCanvasElement;
    await waitFor(() => {
      expect(Number.parseInt(canvas.style.width, 10)).toBeGreaterThan(0);
    });
    const firstWidth = Number.parseInt(canvas.style.width, 10);

    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 640 });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: 480 });
    fireEvent(window, new Event("resize"));

    await waitFor(() => {
      const secondWidth = Number.parseInt(canvas.style.width, 10);
      expect(secondWidth).toBeGreaterThan(0);
      expect(secondWidth).toBeLessThan(firstWidth);
    });

    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: originalInnerWidth });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: originalInnerHeight });
  });

  it("spins reels from keyboard A and updates slot state after animation settles", async () => {
    render(<GameCornerClient />);

    const before = readCoinCount();
    fireEvent.keyDown(window, { key: "z", code: "KeyZ" });

    const spinningState = readState();
    expect(spinningState).toHaveAttribute("data-slot-is-spinning", "true");
    expect(spinningState).toHaveAttribute("data-slot-message", "SPINNING");

    await waitFor(() => {
      expect(readState()).toHaveAttribute("data-slot-is-spinning", "false");
    }, { timeout: 2500 });

    const after = readCoinCount();
    expect(after).toBeGreaterThanOrEqual(0);
    expect(after).toBeLessThanOrEqual(before + 300);

    const state = readState();
    expect(state).toHaveAttribute("data-slot-payout");
    expect(state.getAttribute("data-slot-payout")).not.toBe("");
    expect(state.getAttribute("data-slot-message")).toMatch(/WIN|DARN|NEED MORE COINS/);
  });

  it("changes bet from directional input without exposing a non-ASM slot mode toggle", () => {
    render(<GameCornerClient />);

    const state = readState();
    expect(state).toHaveAttribute("data-slot-bet", "3");
    expect(state).not.toHaveAttribute("data-slot-mode");

    fireEvent.keyDown(window, { code: "ArrowLeft" });
    expect(state).toHaveAttribute("data-slot-bet", "2");

    fireEvent.keyDown(window, { code: "ArrowUp" });
    expect(state).toHaveAttribute("data-slot-bet", "2");
  });

  it("rejects non-slot game corner tabs", () => {
    expect(() => render(<GameCornerClient initialTab={"card-flip" as never} />)).toThrow(
      "Unsupported Game Corner tab: card-flip",
    );
  });
});
