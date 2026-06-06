/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import AudioTestPage from "./page";

jest.mock("./audio-test-client", () => ({
  AudioTestClient: ({ entries }: { entries: Array<{ token: string }> }) => (
    <div data-testid="audio-test-client">{entries.length}</div>
  ),
}));

describe("AudioTestPage", () => {
  it("renders the direct PCM tester shell", () => {
    render(<AudioTestPage />);

    expect(screen.getByTestId("audio-test-page")).toBeInTheDocument();
    expect(screen.getByTestId("audio-test-client")).toBeInTheDocument();
    expect(Number(screen.getByTestId("audio-test-client").textContent)).toBeGreaterThan(250);
  });
});
