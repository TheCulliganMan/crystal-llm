import {
  buildDialogueControlLines,
  buildPromptControlLines,
  type TextSnapshotPayload,
} from "../text-overlays";

type PromptScreenSnapshotOptions = {
  dialogueLines?: string[] | null;
  menuLines?: string[] | null;
  promptLines?: string[] | null;
  infoLines?: string[];
};

export const buildPromptScreenSnapshot = ({
  dialogueLines = null,
  menuLines = null,
  promptLines = null,
  infoLines = [],
}: PromptScreenSnapshotOptions = {}): TextSnapshotPayload => {
  return {
    viewportLines: ["Prompt"],
    infoLines,
    viewportTitle: "Prompt",
    infoTitle: "Legend",
    menuLines,
    promptLines,
    dialogueLines,
  };
};

export const buildPromptCursorLines = (labels: readonly string[], selectedIndex: number): string[] => {
  return labels.map((label, index) => `${index === selectedIndex ? "▶" : "  "} ${label}`);
};

export const DIALOGUE_CONTROL_LINES = buildDialogueControlLines();
export const PROMPT_CONTROL_LINES = buildPromptControlLines();
