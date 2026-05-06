/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import UpdatePasswordPage from "./page";

const mockPush = jest.fn();

jest.mock("next/navigation", () => ({
  useRouter: () => ({ push: mockPush }),
  useSearchParams: () => new URLSearchParams("next=%2Fleaderboard"),
}));

jest.mock("@/components/providers/supabase-provider", () => ({
  useSupabase: () => ({
    supabaseClient: {
      auth: {
        updateUser: jest.fn().mockResolvedValue({ error: null }),
      },
    },
    isConfigured: true,
  }),
}));

describe("UpdatePasswordPage", () => {
  it("renders the password update form inside the suspense boundary", async () => {
    render(<UpdatePasswordPage />);

    expect(await screen.findByRole("heading", { name: "Update password" })).toBeInTheDocument();
    expect(screen.getByLabelText("New password")).toBeInTheDocument();
    expect(screen.getByLabelText("Confirm password")).toBeInTheDocument();
  });
});
