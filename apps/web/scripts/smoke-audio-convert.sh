#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
URL="${1:-http://localhost:3000/audio}"
MIDI_PATH="${2:-$WORKSPACE_DIR/test-fixtures/audio/route29.mid}"
SESSION="audio-smoke-$(date +%s)"
PW=(npx --yes --package @playwright/cli playwright-cli --session "$SESSION")

run_pw() {
  local output
  output="$("${PW[@]}" "$@" 2>&1)"
  echo "$output"
  if echo "$output" | grep -q "### Error"; then
    echo "[smoke-audio] playwright command failed: $*" >&2
    exit 1
  fi
}

cleanup() {
  "${PW[@]}" close >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "[smoke-audio] opening $URL"
run_pw open "$URL" --browser firefox >/dev/null

echo "[smoke-audio] uploading $MIDI_PATH"
run_pw run-code "async (page) => { await page.locator('input[type=\"file\"]').first().setInputFiles('$MIDI_PATH'); }"

echo "[smoke-audio] converting"
run_pw run-code "async (page) => { await page.getByRole('button', { name: 'Convert to MP3' }).click(); await page.getByRole('link', { name: 'Download MP3' }).waitFor({ timeout: 180000 }); }"

echo "[smoke-audio] asserting no init error"
run_pw run-code "async (page) => { const text = await page.textContent('body'); if ((text ?? '').includes('FFMPEG_INIT_FAILED')) { throw new Error('FFMPEG_INIT_FAILED present in UI'); } }"

echo "[smoke-audio] pass"
