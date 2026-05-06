const legendLines = (firstLine: string, ...rest: string[]): string[] => {
  return [firstLine, ...rest];
};

export const buildGenderSelectionControlLines = (confirmed: boolean): string[] =>
  legendLines(confirmed ? "WAIT: applying choice" : "Up/Down=Choose A=Confirm");

export const buildContinueScreenControlLines = (): string[] =>
  legendLines("A=Continue B=Back");

export const buildIntroSequenceControlLines = (finished: boolean): string[] =>
  legendLines(finished ? "WAIT: transitioning to title" : "A/START/SELECT/B=Skip intro");

export const buildOakIntroControlLines = (options: {
  waitingForInput: boolean;
  canRevealText: boolean;
  allowSkip: boolean;
}): string[] => {
  const lines: string[] = [];
  if (options.waitingForInput) {
    lines.push("A/START=Advance");
  } else if (options.canRevealText) {
    lines.push("A/START=Show full text");
  }
  if (options.allowSkip) {
    lines.push("B=Skip intro");
  }
  return lines;
};

export const buildTitleScreenControlLines = (state: "main" | "timeout" | "entrance"): string[] => {
  if (state === "main") {
    return legendLines(
      "A/START=Main menu",
      "Up+B+Select=Delete save",
      "DOWN+B+SELECT arms reset clock"
    );
  }
  if (state === "timeout") {
    return legendLines("WAIT: returning to intro");
  }
  return legendLines("WAIT: title entrance");
};
