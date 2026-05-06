// Learn more: https://github.com/testing-library/jest-dom
require("@testing-library/jest-dom");
const path = require("node:path");

jest.mock("server-only", () => ({}), { virtual: true });

// React 18+/19 act() warning guard.
// Some tests opt into `@jest-environment jsdom` even though the default is `node`.
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

// jsdom does not provide fetch; polyfill it for components that use MCP or other network shims.
if (typeof globalThis.fetch === "undefined") {
  // Keep this lightweight: most jsdom tests just need `fetch` to exist so error
  // paths are exercised deterministically.
  globalThis.fetch = async () => {
    throw new Error("fetch is unavailable in this test environment");
  };
}

if (!process.env.POKECRYSTAL_DISASSEMBLY_ROOT) {
  process.env.POKECRYSTAL_DISASSEMBLY_ROOT = path.resolve(
    __dirname,
    "..",
    "..",
    "vendor",
    "pokecrystal"
  );
}

const buildContext = () => {
  const context = {
    imageSmoothingEnabled: false,
    fillStyle: '',
    strokeStyle: '',
    lineWidth: 1,
    fillRect: jest.fn(),
    clearRect: jest.fn(),
    getImageData: jest.fn((x, y, width, height) => ({
      data: new Uint8ClampedArray(width * height * 4),
      width,
      height,
    })),
    putImageData: jest.fn(),
    createImageData: jest.fn((width, height) => ({
      data: new Uint8ClampedArray(width * height * 4),
      width,
      height,
    })),
    setTransform: jest.fn(),
    drawImage: jest.fn(),
    save: jest.fn(),
    fillText: jest.fn(),
    restore: jest.fn(),
    beginPath: jest.fn(),
    moveTo: jest.fn(),
    lineTo: jest.fn(),
    closePath: jest.fn(),
    stroke: jest.fn(),
    strokeRect: jest.fn(),
    strokeText: jest.fn(),
    fill: jest.fn(),
    translate: jest.fn(),
    scale: jest.fn(),
    rotate: jest.fn(),
    arc: jest.fn(),
    arcTo: jest.fn(),
    ellipse: jest.fn(),
    quadraticCurveTo: jest.fn(),
    bezierCurveTo: jest.fn(),
    rect: jest.fn(),
  };
  return context;
};

class MockOffscreenCanvas {
  constructor(width, height) {
    this.width = width;
    this.height = height;
    this._context = buildContext();
  }

  getContext(type) {
    if (type !== '2d') {
      return null;
    }
    return this._context;
  }
}

if (typeof globalThis.OffscreenCanvas === 'undefined') {
  globalThis.OffscreenCanvas = MockOffscreenCanvas;
}

if (typeof HTMLCanvasElement !== 'undefined') {
  Object.defineProperty(HTMLCanvasElement.prototype, 'getContext', {
    value: buildContext,
  });
}
