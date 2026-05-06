import { redirect } from "next/navigation";
import AudioPage from "@/app/audio/page";

jest.mock("next/navigation", () => ({
  redirect: jest.fn(),
}));

describe("AudioPage", () => {
  it("redirects audio route to game corner audio sub-tab", () => {
    AudioPage();
    expect(redirect).toHaveBeenCalledWith("/game-corner?tab=audio-generation");
  });
});
