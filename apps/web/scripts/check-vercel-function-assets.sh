#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FUNC_DIR="${ROOT_DIR}/.vercel/output/functions/audio/[...path].func"
VC_CONFIG="${FUNC_DIR}/.vc-config.json"

cd "${ROOT_DIR}"

echo "[vercel-check] building Vercel output"
npx vercel build --prod >/dev/null

if [[ ! -d "${FUNC_DIR}" ]]; then
  echo "[vercel-check] missing function dir: ${FUNC_DIR}"
  exit 1
fi

if [[ ! -f "${VC_CONFIG}" ]]; then
  echo "[vercel-check] missing config: ${VC_CONFIG}"
  exit 1
fi

echo "[vercel-check] inspecting function filePathMap"

required_files=(
  "assets/data/pokegear_landmarks.json"
  "assets/gfx/tilesets/bg_tiles.pal"
  "assets/data/map_attributes.json"
)

missing=0
for rel in "${required_files[@]}"; do
  found="$(jq -r --arg rel "$rel" '.filePathMap[$rel] // empty' "${VC_CONFIG}")"
  if [[ -z "${found}" ]]; then
    echo "[vercel-check] missing filePathMap entry: ${rel}"
    missing=1
    continue
  fi
  echo "[vercel-check] mapped: ${rel} -> ${found}"
done

if [[ "${missing}" -ne 0 ]]; then
  echo "[vercel-check] FAIL: function metadata is missing runtime assets (likely deployed 500)."
  exit 1
fi

echo "[vercel-check] PASS"
