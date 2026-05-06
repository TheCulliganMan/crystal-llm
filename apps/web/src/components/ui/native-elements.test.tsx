/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import { Button, CardActionArea, TextField } from "@/components/ui/daisy-elements";

describe("Native UI adapters", () => {
  it("keeps a stable TextField id across rerenders", () => {
    const { rerender } = render(<TextField label="Master Volume" value="0.4" onChange={() => {}} />);

    const firstInput = screen.getByLabelText("Master Volume");
    const firstId = firstInput.getAttribute("id");
    expect(firstId).toBeTruthy();

    rerender(<TextField label="Master Volume" value="0.8" onChange={() => {}} />);

    const secondInput = screen.getByLabelText("Master Volume");
    expect(secondInput).toHaveAttribute("id", firstId ?? "");
  });

  it("does not forward fullWidth to TextField DOM attributes", () => {
    render(<TextField label="Master Volume" value="0.4" onChange={() => {}} fullWidth />);

    const input = screen.getByLabelText("Master Volume");
    expect(input).not.toHaveAttribute("fullWidth");
    expect(input).not.toHaveAttribute("fullwidth");
  });

  it("does not forward fullWidth to Button DOM attributes", () => {
    const consoleErrorSpy = jest.spyOn(console, "error").mockImplementation(() => {});
    try {
      render(
        <Button fullWidth variant="contained">
          Save
        </Button>,
      );

      const button = screen.getByRole("button", { name: "Save" });
      expect(button).not.toHaveAttribute("fullWidth");
      expect(button).toHaveClass("w-full");
      expect(consoleErrorSpy).not.toHaveBeenCalledWith(
        expect.stringContaining("React does not recognize the `fullWidth` prop on a DOM element"),
      );
    } finally {
      consoleErrorSpy.mockRestore();
    }
  });

  it("does not forward fullWidth to CardActionArea DOM attributes", () => {
    render(
      <CardActionArea fullWidth>
        <span>Action target</span>
      </CardActionArea>,
    );

    const button = screen.getByRole("button", { name: /Action target/i });
    expect(button).not.toHaveAttribute("fullWidth");
    expect(button).toHaveClass("w-full");
  });
});
