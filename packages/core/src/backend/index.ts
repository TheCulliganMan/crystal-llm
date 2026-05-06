export {
  BackendSurface,
  BackendWindow,
  BufferFormatSchema,
  ToBytesFormatSchema,
} from "./api";
export type { BackendAdapter, BackendEvent, BufferFormat, RGBAColor, ToBytesFormat } from "./api";
export { getBackend, listBackends } from "./registry";
export { WebBackend } from "./web-backend";
export { Rect, SurfaceController } from "./surface";
