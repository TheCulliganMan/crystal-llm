type TextImageOptions = {
  fontSize?: number;
  lineHeight?: number;
  padding?: number;
  fontFamily?: string;
  background?: string;
  color?: string;
};

const ANSI_ESCAPE_REGEX = /\u001b\[[0-9;]*m/g;
const DEFAULT_FONT_SIZE = 12;
const DEFAULT_LINE_HEIGHT = 14;
const DEFAULT_PADDING = 8;
const DEFAULT_FONT_FAMILY = "monospace";
const DEFAULT_BACKGROUND = "#030312";
const DEFAULT_COLOR = "#e7ebff";

const escapeXml = (value: string): string =>
  value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\"/g, "&quot;");

const sanitizeLines = (text: string): string[] => {
  const cleaned = text.replace(ANSI_ESCAPE_REGEX, "");
  const lines = cleaned.split(/\r?\n/);
  return lines.length ? lines.map((line) => line.replace(/\t/g, "  ")) : [""];
};

export const renderTextSnapshotSvg = (text: string, options: TextImageOptions = {}): string => {
  const fontSize = Math.max(6, Math.floor(options.fontSize ?? DEFAULT_FONT_SIZE));
  const lineHeight = Math.max(fontSize + 2, Math.floor(options.lineHeight ?? DEFAULT_LINE_HEIGHT));
  const padding = Math.max(0, Math.floor(options.padding ?? DEFAULT_PADDING));
  const fontFamily = options.fontFamily ?? DEFAULT_FONT_FAMILY;
  const background = options.background ?? DEFAULT_BACKGROUND;
  const color = options.color ?? DEFAULT_COLOR;
  const lines = sanitizeLines(text);
  const safeLines = lines.map((line) => (line.length ? line : " "));
  const maxLineLength = safeLines.reduce((max, line) => Math.max(max, line.length), 0);
  const charWidth = Math.max(1, Math.round(fontSize * 0.6));
  const width = Math.max(1, padding * 2 + maxLineLength * charWidth);
  const height = Math.max(1, padding * 2 + safeLines.length * lineHeight);
  const textX = padding;
  const textY = padding;

  const tspans = safeLines
    .map((line, index) => {
      const dy = index === 0 ? 0 : lineHeight;
      return `<tspan x="${textX}" dy="${dy}">${escapeXml(line)}</tspan>`;
    })
    .join("");

  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">`,
    `<rect width="100%" height="100%" fill="${background}" />`,
    `<text x="${textX}" y="${textY}" font-family="${fontFamily}" font-size="${fontSize}" fill="${color}" xml:space="preserve" dominant-baseline="hanging">`,
    tspans,
    "</text>",
    "</svg>",
  ].join("");
};
