import { redirect } from "next/navigation";

const AudioPage = () => {
  redirect("/game-corner?tab=audio-generation");
};

export default AudioPage;
