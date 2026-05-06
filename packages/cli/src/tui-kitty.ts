import type { ToolResult } from "./types";

export type GameboyRendererMode = "text" | "kitty";

export type KittyPngFrame = {
  data: string;
  mimeType: "image/png";
  width: number;
  height: number;
};

export type KittyImagePlacement = {
  row: number;
  column: number;
  columns: number;
  rows: number;
};

export type KittyImageDisplay = {
  mode: "placeholder";
  imageId: number;
  placementId: number;
  columns: number;
  rows: number;
  color: string;
};

type WritableTerminal = {
  write: (chunk: string) => unknown;
};

const PNG_SIGNATURE = "89504e470d0a1a0a";
const KITTY_CHUNK_SIZE = 4096;
const KITTY_IMAGE_ID_BASE = 400000;
const KITTY_PLACEMENT_ID = 1;
const KITTY_PLACEHOLDER_CODEPOINT = 0x10eeee;
const KITTY_PLACEHOLDER_MAX_DIMENSION = 256;
const KITTY_ROW_COLUMN_DIACRITICS = [
  0x0305, 0x030d, 0x030e, 0x0310, 0x0312, 0x033d, 0x033e, 0x033f,
  0x0346, 0x034a, 0x034b, 0x034c, 0x0350, 0x0351, 0x0352, 0x0357,
  0x035b, 0x0363, 0x0364, 0x0365, 0x0366, 0x0367, 0x0368, 0x0369,
  0x036a, 0x036b, 0x036c, 0x036d, 0x036e, 0x036f, 0x0483, 0x0484,
  0x0485, 0x0486, 0x0487, 0x0592, 0x0593, 0x0594, 0x0595, 0x0597,
  0x0598, 0x0599, 0x059c, 0x059d, 0x059e, 0x059f, 0x05a0, 0x05a1,
  0x05a8, 0x05a9, 0x05ab, 0x05ac, 0x05af, 0x05c4, 0x0610, 0x0611,
  0x0612, 0x0613, 0x0614, 0x0615, 0x0616, 0x0617, 0x0657, 0x0658,
  0x0659, 0x065a, 0x065b, 0x065d, 0x065e, 0x06d6, 0x06d7, 0x06d8,
  0x06d9, 0x06da, 0x06db, 0x06dc, 0x06df, 0x06e0, 0x06e1, 0x06e2,
  0x06e4, 0x06e7, 0x06e8, 0x06eb, 0x06ec, 0x0730, 0x0732, 0x0733,
  0x0735, 0x0736, 0x073a, 0x073d, 0x073f, 0x0740, 0x0741, 0x0743,
  0x0745, 0x0747, 0x0749, 0x074a, 0x07eb, 0x07ec, 0x07ed, 0x07ee,
  0x07ef, 0x07f0, 0x07f1, 0x07f3, 0x0816, 0x0817, 0x0818, 0x0819,
  0x081b, 0x081c, 0x081d, 0x081e, 0x081f, 0x0820, 0x0821, 0x0822,
  0x0823, 0x0825, 0x0826, 0x0827, 0x0829, 0x082a, 0x082b, 0x082c,
  0x082d, 0x0951, 0x0953, 0x0954, 0x0f82, 0x0f83, 0x0f86, 0x0f87,
  0x135d, 0x135e, 0x135f, 0x17dd, 0x193a, 0x1a17, 0x1a75, 0x1a76,
  0x1a77, 0x1a78, 0x1a79, 0x1a7a, 0x1a7b, 0x1a7c, 0x1b6b, 0x1b6d,
  0x1b6e, 0x1b6f, 0x1b70, 0x1b71, 0x1b72, 0x1b73, 0x1cd0, 0x1cd1,
  0x1cd2, 0x1cda, 0x1cdb, 0x1ce0, 0x1dc0, 0x1dc1, 0x1dc3, 0x1dc4,
  0x1dc5, 0x1dc6, 0x1dc7, 0x1dc8, 0x1dc9, 0x1dcb, 0x1dcc, 0x1dd1,
  0x1dd2, 0x1dd3, 0x1dd4, 0x1dd5, 0x1dd6, 0x1dd7, 0x1dd8, 0x1dd9,
  0x1dda, 0x1ddb, 0x1ddc, 0x1ddd, 0x1dde, 0x1ddf, 0x1de0, 0x1de1,
  0x1de2, 0x1de3, 0x1de4, 0x1de5, 0x1de6, 0x1dfe, 0x20d0, 0x20d1,
  0x20d4, 0x20d5, 0x20d6, 0x20d7, 0x20db, 0x20dc, 0x20e1, 0x20e7,
  0x20e9, 0x20f0, 0x2cef, 0x2cf0, 0x2cf1, 0x2de0, 0x2de1, 0x2de2,
  0x2de3, 0x2de4, 0x2de5, 0x2de6, 0x2de7, 0x2de8, 0x2de9, 0x2dea,
  0x2deb, 0x2dec, 0x2ded, 0x2dee, 0x2def, 0x2df0, 0x2df1, 0x2df2,
  0x2df3, 0x2df4, 0x2df5, 0x2df6, 0x2df7, 0x2df8, 0x2df9, 0x2dfa,
  0x2dfb, 0x2dfc, 0x2dfd, 0x2dfe, 0x2dff, 0xa66f, 0xa67c, 0xa67d,
  0xa6f0, 0xa6f1, 0xa8e0, 0xa8e1, 0xa8e2, 0xa8e3, 0xa8e4, 0xa8e5,
  0xa8e6, 0xa8e7,
];

const sanitizeBase64 = (value: string): string => value.replace(/\s+/g, "");

export const extractKittyPngFrame = (result?: ToolResult): KittyPngFrame | null => {
  const image = (result?.content ?? []).find(
    (entry) => entry.type === "image" && entry.mimeType === "image/png" && typeof entry.data === "string",
  );
  if (!image?.data) {
    return null;
  }
  const data = sanitizeBase64(image.data);
  let bytes: Buffer;
  try {
    bytes = Buffer.from(data, "base64");
  } catch {
    return null;
  }
  if (bytes.length < 24 || bytes.subarray(0, 8).toString("hex") !== PNG_SIGNATURE) {
    return null;
  }
  return {
    data,
    mimeType: "image/png",
    width: bytes.readUInt32BE(16),
    height: bytes.readUInt32BE(20),
  };
};

export const isKittyGraphicsSupported = (
  env: NodeJS.ProcessEnv = process.env,
): boolean => {
  const override = env.POKECRYSTAL_CLI_KITTY?.trim();
  if (override === "1" || override?.toLowerCase() === "true") {
    return true;
  }
  if (override === "0" || override?.toLowerCase() === "false") {
    return false;
  }
  const termProgram = String(env.TERM_PROGRAM ?? "").toLowerCase();
  const term = String(env.TERM ?? "").toLowerCase();
  return (
    termProgram.includes("ghostty") ||
    termProgram.includes("kitty") ||
    Boolean(env.KITTY_WINDOW_ID) ||
    Boolean(env.GHOSTTY_RESOURCES_DIR) ||
    term.includes("ghostty") ||
    term.includes("xterm-kitty")
  );
};

export const isKittyPlaceholderModeEnabled = (
  env: NodeJS.ProcessEnv = process.env,
  graphicsSupported = isKittyGraphicsSupported(env),
): boolean => {
  const override = env.POKECRYSTAL_CLI_KITTY_PLACEHOLDERS?.trim().toLowerCase();
  if (override === "1" || override === "true") {
    return true;
  }
  if (override === "0" || override === "false") {
    return false;
  }
  return graphicsSupported;
};

const clampPositiveInt = (value: number, fallback: number): number =>
  Number.isFinite(value) && value > 0 ? Math.floor(value) : fallback;

const clampPlaceholderDimension = (value: number, fallback: number): number =>
  Math.max(1, Math.min(KITTY_PLACEHOLDER_MAX_DIMENSION, clampPositiveInt(value, fallback)));

export const buildKittyDeleteSequence = (imageId: number): string =>
  `\u001b_Ga=d,d=I,i=${imageId},q=2;\u001b\\`;

const buildCursorMoveSequence = (placement: KittyImagePlacement): string =>
  `\u001b[${clampPositiveInt(placement.row, 1)};${clampPositiveInt(placement.column, 1)}H`;

export const resolveKittyImageId = (sessionId: string): number => {
  let hash = 0;
  for (let index = 0; index < sessionId.length; index += 1) {
    hash = ((hash * 31) + sessionId.charCodeAt(index)) >>> 0;
  }
  return KITTY_IMAGE_ID_BASE + (hash % 500000);
};

export const resolveKittyImageIds = (sessionId: string): [number, number] => {
  const base = resolveKittyImageId(sessionId);
  return [base, base + 1];
};

export const buildKittyPlaceholderColor = (imageId: number): string =>
  `#${(imageId & 0xffffff).toString(16).padStart(6, "0")}`;

export const KITTY_PLACEHOLDER_CELL = String.fromCodePoint(KITTY_PLACEHOLDER_CODEPOINT);

const buildKittyPlaceholderMark = (value: number): string =>
  String.fromCodePoint(KITTY_ROW_COLUMN_DIACRITICS[
    Math.max(0, Math.min(KITTY_ROW_COLUMN_DIACRITICS.length - 1, Math.floor(value)))
  ]);

export const buildKittyPlaceholderLines = (columns: number, rows: number): string[] => {
  const safeColumns = clampPlaceholderDimension(columns, 20);
  const safeRows = clampPlaceholderDimension(rows, 10);
  return Array.from({ length: safeRows }, (_value, row) =>
    `${KITTY_PLACEHOLDER_CELL}${buildKittyPlaceholderMark(row)}${KITTY_PLACEHOLDER_CELL.repeat(safeColumns - 1)}`
  );
};

const chunkKittyPayload = (data: string, controls: string[]): string => {
  const chunks = data.match(new RegExp(`.{1,${KITTY_CHUNK_SIZE}}`, "g")) ?? [""];
  return chunks.map((chunk, index) => {
    const more = index < chunks.length - 1 ? 1 : 0;
    const prefix = index === 0 ? `${controls.join(",")},m=${more}` : `m=${more}`;
    return `\u001b_G${prefix};${chunk}\u001b\\`;
  }).join("");
};

export const buildKittyUploadSequence = (
  frame: KittyPngFrame,
  imageId: number,
): string => {
  return chunkKittyPayload(frame.data, [`a=t`, `f=100`, `i=${imageId}`, `q=2`]);
};

export const buildKittyTransmitSequence = (
  frame: KittyPngFrame,
  placement: KittyImagePlacement,
  imageId: number,
  placementId = KITTY_PLACEMENT_ID,
): string => chunkKittyPayload(frame.data, [
  `a=T`,
  `f=100`,
  `i=${imageId}`,
  `p=${placementId}`,
  `q=2`,
  `c=${clampPositiveInt(placement.columns, 20)}`,
  `r=${clampPositiveInt(placement.rows, 10)}`,
  `C=1`,
]);

export const buildKittyVirtualPlacementSequence = (
  placement: KittyImagePlacement,
  imageId: number,
  placementId = KITTY_PLACEMENT_ID,
): string =>
  `\u001b_Ga=p,U=1,i=${imageId},p=${placementId},q=2,c=${clampPlaceholderDimension(placement.columns, 20)},r=${clampPlaceholderDimension(placement.rows, 10)};\u001b\\`;

export const buildKittyPlaceSequence = (
  placement: KittyImagePlacement,
  imageId: number,
  placementId = KITTY_PLACEMENT_ID,
): string => [
  buildCursorMoveSequence(placement),
  `\u001b_Ga=p,i=${imageId},p=${placementId},q=2,c=${clampPositiveInt(placement.columns, 20)},r=${clampPositiveInt(placement.rows, 10)},C=1;\u001b\\`,
].join("");

export const buildKittyFrameSequence = (
  frame: KittyPngFrame,
  placement: KittyImagePlacement,
  imageId: number,
  placementId = KITTY_PLACEMENT_ID,
): string => [
  buildCursorMoveSequence(placement),
  buildKittyTransmitSequence(frame, placement, imageId, placementId),
].join("");

export type KittyImageRenderer = {
  usesPlaceholders: boolean;
  update: (
    frame: KittyPngFrame | null | undefined,
    placement: KittyImagePlacement | null | undefined,
  ) => KittyImageDisplay | undefined;
  commit: () => void;
  redraw: () => void;
  clear: () => void;
};

type KittyImageSlot = {
  imageId: number;
  uploaded: boolean;
  frameData: string;
  width: number;
  height: number;
  placementKey: string;
  placement: KittyImagePlacement | null;
};

const createSlot = (imageId: number): KittyImageSlot => ({
  imageId,
  uploaded: false,
  frameData: "",
  width: 0,
  height: 0,
  placementKey: "",
  placement: null,
});

const placementKeyFor = (placement: KittyImagePlacement, placeholders: boolean): string =>
  placeholders
    ? `${clampPlaceholderDimension(placement.columns, 20)},${clampPlaceholderDimension(placement.rows, 10)}`
    : `${clampPositiveInt(placement.row, 1)},${clampPositiveInt(placement.column, 1)},${clampPositiveInt(placement.columns, 20)},${clampPositiveInt(placement.rows, 10)}`;

const displayForSlot = (slot: KittyImageSlot, placement: KittyImagePlacement): KittyImageDisplay => ({
  mode: "placeholder",
  imageId: slot.imageId,
  placementId: KITTY_PLACEMENT_ID,
  columns: clampPlaceholderDimension(placement.columns, 20),
  rows: clampPlaceholderDimension(placement.rows, 10),
  color: buildKittyPlaceholderColor(slot.imageId),
});

export const createKittyImageRenderer = (
  terminal: WritableTerminal,
  options: { supported?: boolean; imageId?: number; imageIds?: [number, number]; placeholderMode?: boolean } = {},
): KittyImageRenderer => {
  const supported = options.supported ?? isKittyGraphicsSupported();
  const usesPlaceholders =
    supported && (options.placeholderMode ?? isKittyPlaceholderModeEnabled(process.env, supported));
  const imageIds = options.imageIds ?? (() => {
    const base = options.imageId ?? resolveKittyImageId("default");
    return [base, base + 1] as [number, number];
  })();
  const slots = imageIds.map(createSlot);
  let activeSlotIndex: number | null = null;
  const pendingDeleteImageIds = new Set<number>();

  const deleteImage = (imageId: number): void => {
    terminal.write(buildKittyDeleteSequence(imageId));
    for (const slot of slots) {
      if (slot.imageId === imageId) {
        slot.uploaded = false;
        slot.frameData = "";
        slot.width = 0;
        slot.height = 0;
        slot.placementKey = "";
        slot.placement = null;
      }
    }
  };

  const queueInactiveDelete = (imageId: number): void => {
    const activeImageId = activeSlotIndex === null ? null : slots[activeSlotIndex]?.imageId;
    if (imageId !== activeImageId) {
      pendingDeleteImageIds.add(imageId);
    }
  };

  const writePlacement = (
    slot: KittyImageSlot,
    placement: KittyImagePlacement,
  ): void => {
    if (usesPlaceholders) {
      terminal.write(buildKittyVirtualPlacementSequence(placement, slot.imageId, KITTY_PLACEMENT_ID));
      return;
    }
    terminal.write(buildKittyPlaceSequence(placement, slot.imageId, KITTY_PLACEMENT_ID));
  };

  const uploadAndPlace = (
    slot: KittyImageSlot,
    frame: KittyPngFrame,
    placement: KittyImagePlacement,
  ): void => {
    if (usesPlaceholders) {
      terminal.write(buildKittyUploadSequence(frame, slot.imageId));
      terminal.write(buildKittyVirtualPlacementSequence(placement, slot.imageId, KITTY_PLACEMENT_ID));
    } else {
      terminal.write(buildKittyFrameSequence(frame, placement, slot.imageId, KITTY_PLACEMENT_ID));
    }
    slot.uploaded = true;
    slot.frameData = frame.data;
    slot.width = frame.width;
    slot.height = frame.height;
    slot.placementKey = placementKeyFor(placement, usesPlaceholders);
    slot.placement = placement;
  };

  const clear = (): void => {
    if (!supported) {
      return;
    }
    for (const slot of slots) {
      if (slot.uploaded || pendingDeleteImageIds.has(slot.imageId)) {
        deleteImage(slot.imageId);
      }
    }
    pendingDeleteImageIds.clear();
    activeSlotIndex = null;
  };

  const update = (
    frame: KittyPngFrame | null | undefined,
    placement: KittyImagePlacement | null | undefined,
  ): KittyImageDisplay | undefined => {
    if (!supported) {
      return undefined;
    }
    if (!frame || !placement) {
      for (const slot of slots) {
        if (slot.uploaded) {
          pendingDeleteImageIds.add(slot.imageId);
        }
      }
      activeSlotIndex = null;
      return undefined;
    }

    const activeSlot = activeSlotIndex === null ? null : slots[activeSlotIndex];
    const sameFrame =
      Boolean(activeSlot?.uploaded) &&
      activeSlot?.frameData === frame.data &&
      activeSlot?.width === frame.width &&
      activeSlot?.height === frame.height;
    const nextPlacementKey = placementKeyFor(placement, usesPlaceholders);

    if (activeSlot && sameFrame) {
      if (activeSlot.placementKey !== nextPlacementKey) {
        writePlacement(activeSlot, placement);
        activeSlot.placementKey = nextPlacementKey;
        activeSlot.placement = placement;
      }
      return usesPlaceholders ? displayForSlot(activeSlot, placement) : undefined;
    }

    const previousSlotIndex = activeSlotIndex;
    const nextSlotIndex = activeSlotIndex === 0 ? 1 : 0;
    const nextSlot = slots[nextSlotIndex];
    uploadAndPlace(nextSlot, frame, placement);
    activeSlotIndex = nextSlotIndex;
    if (previousSlotIndex !== null && previousSlotIndex !== nextSlotIndex) {
      queueInactiveDelete(slots[previousSlotIndex].imageId);
    }
    return usesPlaceholders ? displayForSlot(nextSlot, placement) : undefined;
  };

  return {
    usesPlaceholders,
    update,
    commit: () => {
      if (!supported) {
        pendingDeleteImageIds.clear();
        return;
      }
      for (const imageId of [...pendingDeleteImageIds]) {
        const activeImageId = activeSlotIndex === null ? null : slots[activeSlotIndex]?.imageId;
        if (imageId !== activeImageId) {
          deleteImage(imageId);
        }
        pendingDeleteImageIds.delete(imageId);
      }
    },
    redraw: () => {
      const activeSlot = activeSlotIndex === null ? null : slots[activeSlotIndex];
      if (!supported || !activeSlot?.uploaded || !activeSlot.placement) {
        return;
      }
      writePlacement(activeSlot, activeSlot.placement);
    },
    clear,
  };
};
