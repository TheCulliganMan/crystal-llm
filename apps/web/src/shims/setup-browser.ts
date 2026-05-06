import { Buffer as NodeBuffer } from "buffer";

if (typeof globalThis !== "undefined" && !("Buffer" in globalThis)) {
  (globalThis as typeof globalThis & { Buffer?: typeof NodeBuffer }).Buffer = NodeBuffer;
}
