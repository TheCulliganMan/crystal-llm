import {
  ASM_SCREEN_HEIGHT_PX,
  ASM_SCREEN_WIDTH_PX,
  ASM_TILE_SIZE_PX,
  assertAsmScale,
  assertAsmScreenDimensions,
  assertAsmTileGeometry,
} from "@/app/asm-rendering-invariants";

describe("asm rendering invariants", () => {
  it("accepts canonical Crystal geometry", () => {
    expect(() => assertAsmScreenDimensions(ASM_SCREEN_WIDTH_PX, ASM_SCREEN_HEIGHT_PX, "test")).not.toThrow();
    expect(() => assertAsmTileGeometry(ASM_TILE_SIZE_PX, "test")).not.toThrow();
    expect(() => assertAsmScale(1, "test")).not.toThrow();
    expect(() => assertAsmScale(2, "test")).not.toThrow();
  });

  it("rejects non-ASM geometry", () => {
    expect(() => assertAsmScreenDimensions(161, ASM_SCREEN_HEIGHT_PX, "test")).toThrow("Expected ASM screen");
    expect(() => assertAsmTileGeometry(16, "test")).toThrow("Expected ASM tile size");
    expect(() => assertAsmScale(1.5, "test")).toThrow("Scale must be a positive integer");
  });
});

