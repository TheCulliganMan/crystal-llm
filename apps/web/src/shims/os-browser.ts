export function homedir(): string {
  return "";
}

export function tmpdir(): string {
  return "";
}

export function platform(): string {
  return "browser";
}

export function release(): string {
  return "";
}

const osBrowser = {
  homedir,
  tmpdir,
  platform,
  release,
};

export default osBrowser;
