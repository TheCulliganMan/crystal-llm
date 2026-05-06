/** @jest-environment jsdom */
import { WebBackend } from "./web-backend";

type StubOffscreenContext = {
  putImageData: jest.Mock<void, [ImageData, number, number]>;
};

class StubOffscreenCanvas {
  public width: number;
  public height: number;
  public readonly context: StubOffscreenContext;
  public readonly convertToBlob: jest.Mock<Promise<Blob | null>, [BlobPropertyBag]>;

  constructor(width: number, height: number) {
    this.width = width;
    this.height = height;
    this.context = {
      putImageData: jest.fn(),
    };
    this.convertToBlob = jest.fn(async () =>
      new Blob(["stub"], { type: "image/png" })
    );
  }

  getContext(): StubOffscreenContext {
    return this.context;
  }
}

describe("WebBackend.savePng", () => {
  let originalOffscreenCanvas: undefined | (typeof OffscreenCanvas);
  let originalCreateElement: (tagName: string) => HTMLElement;
  let originalCreateObjectURL: ((object: Blob | MediaSource) => string) | undefined;
  let originalRevokeObjectURL: ((url: string) => void) | undefined;
  let createdAnchor: HTMLAnchorElement | null = null;
  let createObjectURLSpy: jest.Mock<string, [Blob | MediaSource]>;
  let revokeObjectURLSpy: jest.Mock<void, [string]>;

  beforeEach(() => {
    originalOffscreenCanvas = (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas;
    (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas = StubOffscreenCanvas as typeof OffscreenCanvas;
    originalCreateElement = document.createElement.bind(document);
    originalCreateObjectURL = URL.createObjectURL;
    originalRevokeObjectURL = URL.revokeObjectURL;

    createdAnchor = null;
    createObjectURLSpy = jest.fn(() => "blob:mock-png");
    revokeObjectURLSpy = jest.fn();
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: createObjectURLSpy,
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: revokeObjectURLSpy,
    });

    jest.spyOn(document, "createElement").mockImplementation((tagName: string) => {
      const node = originalCreateElement(tagName);
      if (tagName === "a") {
        createdAnchor = node as HTMLAnchorElement;
      }
      return node;
    });
  });

  afterEach(() => {
    jest.restoreAllMocks();
    (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas =
      originalOffscreenCanvas;
    if (originalCreateObjectURL) {
      Object.defineProperty(URL, "createObjectURL", {
        configurable: true,
        value: originalCreateObjectURL,
      });
    } else {
      delete (URL as Partial<typeof URL>).createObjectURL;
    }
    if (originalRevokeObjectURL) {
      Object.defineProperty(URL, "revokeObjectURL", {
        configurable: true,
        value: originalRevokeObjectURL,
      });
    } else {
      delete (URL as Partial<typeof URL>).revokeObjectURL;
    }
  });

  it("exports the surface to a browser download", async () => {
    const backend = new WebBackend({ headless: true });
    const surface = backend.createSurface(2, 2);
    backend.fill(surface, [255, 0, 0, 255]);
    const clickSpy = jest.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      expect(this).toBe(createdAnchor);
    });

    await expect(backend.savePng(surface, "dump/pokedex")).resolves.toBeUndefined();

    expect(createdAnchor).not.toBeNull();
    expect(createdAnchor?.getAttribute("download")).toBe("pokedex.png");
    expect(clickSpy).toHaveBeenCalledTimes(1);
    expect(createObjectURLSpy).toHaveBeenCalled();
    expect(revokeObjectURLSpy).toHaveBeenCalledWith("blob:mock-png");
    expect((createdAnchor as HTMLAnchorElement).href).toBe("blob:mock-png");
  });

  it("normalizes destination names to PNG filenames", async () => {
    const backend = new WebBackend({ headless: true });
    const surface = backend.createSurface(1, 1);
    backend.fill(surface, [0, 0, 0, 255]);
    const clickSpy = jest.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      expect(this).toBe(createdAnchor);
    });

    await expect(backend.savePng(surface, "capture.png")).resolves.toBeUndefined();

    expect(createdAnchor?.getAttribute("download")).toBe("capture.png");
    expect(clickSpy).toHaveBeenCalledTimes(1);
  });
});

describe("WebBackend.present", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("reuses the same ImageData object for RGBA surfaces across frames", () => {
    const context = {
      putImageData: jest.fn<void, [ImageData, number, number]>(),
      clearRect: jest.fn<void, [number, number, number, number]>(),
      drawImage: jest.fn<void, [CanvasImageSource, number, number, number, number]>(),
      imageSmoothingEnabled: false,
    };
    jest
      .spyOn(HTMLCanvasElement.prototype, "getContext")
      .mockReturnValue(context as unknown as CanvasRenderingContext2D);

    const backend = new WebBackend({ headless: false, inputEnabled: false });
    const win = backend.createWindow(2, 2, { scale: 1, headless: false });
    const surface = backend.createSurface(2, 2);
    backend.fill(surface, [255, 0, 0, 255]);

    const frames = 2_000;
    for (let i = 0; i < frames; i += 1) {
      backend.present(win, surface);
    }

    expect(context.putImageData).toHaveBeenCalledTimes(frames);
    const firstFrameImageData = context.putImageData.mock.calls[0]?.[0];
    const secondFrameImageData = context.putImageData.mock.calls[1]?.[0];
    expect(firstFrameImageData).toBeDefined();
    expect(firstFrameImageData).toBe(secondFrameImageData);
  });

  it("reuses indexed ImageData and refreshes pixels when palette/index data changes", () => {
    const context = {
      putImageData: jest.fn<void, [ImageData, number, number]>(),
      clearRect: jest.fn<void, [number, number, number, number]>(),
      drawImage: jest.fn<void, [CanvasImageSource, number, number, number, number]>(),
      imageSmoothingEnabled: false,
    };
    jest
      .spyOn(HTMLCanvasElement.prototype, "getContext")
      .mockReturnValue(context as unknown as CanvasRenderingContext2D);

    const backend = new WebBackend({ headless: false, inputEnabled: false });
    const win = backend.createWindow(2, 2, { scale: 1, headless: false });
    const surface = backend.createSurface(2, 2, { indexed: true });
    backend.setPalette(surface, [
      [0, 0, 0, 255],
      [255, 0, 0, 255],
      [0, 255, 0, 255],
    ]);
    backend.fill(surface, [255, 0, 0, 255]);

    backend.present(win, surface);
    backend.present(win, surface);

    const firstFrameImageData = context.putImageData.mock.calls[0]?.[0];
    const secondFrameImageData = context.putImageData.mock.calls[1]?.[0];
    expect(firstFrameImageData).toBeDefined();
    expect(firstFrameImageData).toBe(secondFrameImageData);
    expect(Array.from(firstFrameImageData?.data.slice(0, 4) ?? [])).toEqual([255, 0, 0, 255]);

    backend.fill(surface, [0, 255, 0, 255]);
    backend.present(win, surface);

    const thirdFrameImageData = context.putImageData.mock.calls[2]?.[0];
    expect(thirdFrameImageData).toBe(firstFrameImageData);
    expect(Array.from(thirdFrameImageData?.data.slice(0, 4) ?? [])).toEqual([0, 255, 0, 255]);

    backend.setPalette(surface, [
      [0, 0, 0, 255],
      [255, 0, 0, 255],
      [0, 0, 255, 255],
    ]);
    backend.present(win, surface);

    const fourthFrameImageData = context.putImageData.mock.calls[3]?.[0];
    expect(fourthFrameImageData).toBe(firstFrameImageData);
    expect(Array.from(fourthFrameImageData?.data.slice(0, 4) ?? [])).toEqual([0, 0, 255, 255]);
  });
});

describe("WebBackend.blit", () => {
  it("copies indexed pixels directly for indexed destinations", () => {
    const backend = new WebBackend({ headless: true, inputEnabled: false });
    const dest = backend.createSurface(2, 2, { indexed: true });
    const src = backend.createSurface(2, 2, { indexed: true });
    const palette: ReadonlyArray<ReadonlyArray<number>> = [
      [0, 0, 0, 255],
      [255, 0, 0, 255],
      [0, 255, 0, 255],
    ];
    backend.setPalette(dest, palette);
    backend.setPalette(src, palette);
    backend.fill(dest, [0, 0, 0, 255]);
    backend.fill(src, [0, 255, 0, 255]);

    backend.blit(dest, src, [0, 0, 2, 2]);

    expect(backend.getPixel(dest, 0, 0)).toEqual([0, 255, 0, 255]);
    expect(backend.getPixel(dest, 1, 1)).toEqual([0, 255, 0, 255]);
  });

  it("honors indexed colorkey when blitting to RGBA surfaces", () => {
    const backend = new WebBackend({ headless: true, inputEnabled: false });
    const dest = backend.createSurface(2, 1);
    const src = backend.createSurface(2, 1, { indexed: true });
    backend.setPalette(src, [
      [0, 0, 0, 255],
      [255, 0, 0, 255],
    ]);
    backend.fill(dest, [0, 0, 255, 255]);
    backend.fill(src, [255, 0, 0, 255]);
    backend.setPixel(src, 0, 0, [0, 0, 0, 255]);
    backend.setColorkey(src, [0, 0, 0, 255]);

    backend.blit(dest, src, [0, 0, 2, 1]);

    expect(backend.getPixel(dest, 0, 0)).toEqual([0, 0, 255, 255]);
    expect(backend.getPixel(dest, 1, 0)).toEqual([255, 0, 0, 255]);
  });

  it("clips blits with negative destination offsets", () => {
    const backend = new WebBackend({ headless: true, inputEnabled: false });
    const dest = backend.createSurface(2, 2);
    const src = backend.createSurface(2, 2);
    backend.fill(dest, [0, 0, 0, 255]);
    backend.fill(src, [255, 255, 255, 255]);

    backend.blit(dest, src, [-1, -1, 2, 2]);

    expect(backend.getPixel(dest, 0, 0)).toEqual([255, 255, 255, 255]);
    expect(backend.getPixel(dest, 1, 0)).toEqual([0, 0, 0, 255]);
    expect(backend.getPixel(dest, 0, 1)).toEqual([0, 0, 0, 255]);
    expect(backend.getPixel(dest, 1, 1)).toEqual([0, 0, 0, 255]);
  });
});
