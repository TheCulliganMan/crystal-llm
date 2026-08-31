import { redirect } from "next/navigation";
import AudioPage from "@/app/audio/page";

jest.mock("next/navigation", () => ({
  redirect: jest.fn(),
}));

describe("AudioPage", () => {
  it("redirects audio route to the canonical PCM catalog", () => {
    AudioPage();
    expect(redirect).toHaveBeenCalledWith("/audio-test");
  });
});
