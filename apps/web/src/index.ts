import "@/lib/pokecrystal-core/register-browser-adapters";
import {
  AudioEngine,
  type KeyEvent,
  GB_FRAME_DURATION_MS,
  IntroSequence,
  TitleScreen,
} from "@pokecrystal/core";

const FRAME_DURATION_MS = GB_FRAME_DURATION_MS;
const MAX_ACCUMULATED_MS = FRAME_DURATION_MS * 5;

const CANVAS_WIDTH =
  Math.max(IntroSequence.SCREEN_WIDTH, TitleScreen.SCREEN_WIDTH_TILES) *
  IntroSequence.TILE_SIZE;
const CANVAS_HEIGHT =
  Math.max(IntroSequence.SCREEN_HEIGHT, TitleScreen.SCREEN_HEIGHT_TILES) *
  IntroSequence.TILE_SIZE;

type ActiveScreen = "intro" | "title";

const toKeyEvent = (event: KeyboardEvent, isPress: boolean): KeyEvent => ({
  type: isPress ? "keydown" : "keyup",
  key: event.key,
  code: event.code,
  is_press: isPress,
});

async function main() {
  const canvas = document.getElementById("game") as HTMLCanvasElement;
  if (!canvas) {
    console.error("Canvas not found");
    return;
  }

  const ctx = canvas.getContext("2d");
  if (!ctx) {
    console.error("Could not get canvas context");
    return;
  }

  canvas.width = CANVAS_WIDTH;
  canvas.height = CANVAS_HEIGHT;

  const audioEngine = new AudioEngine();
  const introSequence = new IntroSequence(audioEngine);
  const titleScreen = await TitleScreen.create(audioEngine);

  let screen: ActiveScreen = "intro";

  const handleKeyboard = (event: KeyboardEvent, isPress: boolean): void => {
    if (screen === "intro") {
      const inputEvent = toKeyEvent(event, isPress);
      if (isPress && introSequence.handleInput(inputEvent)) {
        titleScreen.startFromGameStart();
        screen = "title";
      }
      return;
    }

    titleScreen.handleInput(event, isPress);
  };

  window.addEventListener("keydown", (event) => {
    handleKeyboard(event, true);
  });
  window.addEventListener("keyup", (event) => {
    handleKeyboard(event, false);
  });

  let lastFrameTime: number | null = null;
  let frameRemainderMs = 0;
  const gameLoop = (timestamp: number) => {
    if (lastFrameTime === null) {
      lastFrameTime = timestamp;
      frameRemainderMs = FRAME_DURATION_MS;
    } else {
      const delta = Math.max(0, timestamp - lastFrameTime);
      lastFrameTime = timestamp;
      frameRemainderMs = Math.min(
        frameRemainderMs + delta,
        MAX_ACCUMULATED_MS
      );
    }

    const framesToProcess = Math.floor(frameRemainderMs / FRAME_DURATION_MS);
    if (framesToProcess > 0) {
      const capped = Math.min(
        framesToProcess,
        Math.floor(MAX_ACCUMULATED_MS / FRAME_DURATION_MS)
      );
      frameRemainderMs -= capped * FRAME_DURATION_MS;

      for (let i = 0; i < capped; i += 1) {
        audioEngine.update();
        if (screen === "intro") {
          const done = introSequence.update();
          introSequence.draw(ctx);
          if (done) {
            titleScreen.startFromGameStart();
            screen = "title";
          }
          continue;
        }

        titleScreen.update();
        titleScreen.draw(ctx);
        const action = titleScreen.popAction();
        if (action) {
          console.log(`Action: ${action}`);
        }
      }
    }

    requestAnimationFrame(gameLoop);
  };

  requestAnimationFrame(gameLoop);
}

main();
