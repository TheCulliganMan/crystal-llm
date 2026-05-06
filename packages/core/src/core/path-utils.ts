const NORMALIZE_SLASHES = /\\/g;
const COLLAPSE_SLASHES = /\/{2,}/g;

export const normalizePath = (value: string): string =>
  value.replace(NORMALIZE_SLASHES, "/").replace(COLLAPSE_SLASHES, "/");

export const joinPath = (...parts: string[]): string => {
  const filtered = parts.filter(Boolean);
  if (!filtered.length) {
    return "";
  }
  const joined = normalizePath(filtered.join("/"));
  if (filtered[0].startsWith("/") && !joined.startsWith("/")) {
    return `/${joined}`;
  }
  return joined;
};
