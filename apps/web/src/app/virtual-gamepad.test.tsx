/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";

jest.mock("next/font/google", () => ({
  Space_Grotesk: () => ({ className: "space-grotesk", style: { fontFamily: "Space Grotesk" } }),
  Space_Mono: () => ({ className: "space-mono", style: { fontFamily: "Space Mono" } }),
}));

import { VirtualGamepad } from "./virtual-gamepad";

describe("VirtualGamepad", () => {
  const postEvent = jest.fn();

  beforeEach(() => {
    postEvent.mockReset();
  });

  it("renders the play bar heading and control sections by default", () => {
    render(<VirtualGamepad pressedButtons={[]} pressedKeys={[]} postEvent={postEvent} />);

    expect(screen.getByText("Play bar")).toBeInTheDocument();
    expect(screen.getByText("D-pad")).toBeInTheDocument();
    expect(screen.getByText("Action")).toBeInTheDocument();
    expect(screen.getByText("System")).toBeInTheDocument();
    expect(screen.getByText("Pressed")).toBeInTheDocument();
    expect(screen.getByText("Held keys")).toBeInTheDocument();
  });

  it("supports embedded mode and custom system controls", () => {
    render(
      <VirtualGamepad
        pressedButtons={[]}
        pressedKeys={[]}
        postEvent={postEvent}
        embedded
        showHeader={false}
        systemControl={<button type="button">Toggle View</button>}
      />
    );

    expect(screen.queryByText("Play bar")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Toggle View" })).toBeInTheDocument();
  });

  it("keeps compact embedded controls available", () => {
    render(
      <VirtualGamepad
        pressedButtons={[]}
        pressedKeys={[]}
        postEvent={postEvent}
        embedded
        compact
        showHeader={false}
        systemControl={<button type="button">Toggle View</button>}
      />
    );

    expect(screen.getByRole("button", { name: "Select button" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start button" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Toggle View" })).toBeInTheDocument();
  });
});
