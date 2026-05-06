#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${PORT:-3110}"
SESSION_ID="local-smoke-$(date +%s)"
START_LOG="${ROOT_DIR}/.next/local-smoke-start.log"

cd "${ROOT_DIR}"

echo "[smoke] building app"
npm run build >/dev/null

echo "[smoke] starting app on port ${PORT}"
PORT="${PORT}" npm run start >"${START_LOG}" 2>&1 &
APP_PID=$!

cleanup() {
  kill "${APP_PID}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

for _ in $(seq 1 80); do
  if curl -fsS "http://127.0.0.1:${PORT}/" >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

echo "[smoke] register_identity"
REGISTER_JSON="$(curl -fsS -X POST "http://127.0.0.1:${PORT}/api/mcp/tools?session_id=${SESSION_ID}" \
  -H "content-type: application/json" \
  -d '{"tool":"register_identity","arguments":{"name":"LocalSmoke"}}')"

if [[ "$(printf '%s' "${REGISTER_JSON}" | jq -r '.ok')" != "true" ]]; then
  echo "[smoke] register_identity failed: ${REGISTER_JSON}"
  exit 1
fi

TOKEN="$(printf '%s' "${REGISTER_JSON}" | jq -r '.result.content[0].text | fromjson | .token')"
if [[ -z "${TOKEN}" || "${TOKEN}" == "null" ]]; then
  echo "[smoke] missing token in register_identity response"
  exit 1
fi

echo "[smoke] observe"
OBSERVE_JSON="$(curl -fsS -X POST "http://127.0.0.1:${PORT}/api/mcp/tools?session_id=${SESSION_ID}" \
  -H "content-type: application/json" \
  -H "authorization: Bearer ${TOKEN}" \
  -d '{"tool":"observe","arguments":{}}')"

if [[ "$(printf '%s' "${OBSERVE_JSON}" | jq -r '.ok')" != "true" ]]; then
  echo "[smoke] observe failed: ${OBSERVE_JSON}"
  exit 1
fi

echo "[smoke] status"
STATUS_JSON="$(curl -fsS -X POST "http://127.0.0.1:${PORT}/api/mcp/tools?session_id=${SESSION_ID}" \
  -H "content-type: application/json" \
  -H "authorization: Bearer ${TOKEN}" \
  -d '{"tool":"status","arguments":{}}')"

if [[ "$(printf '%s' "${STATUS_JSON}" | jq -r '.ok')" != "true" ]]; then
  echo "[smoke] status failed: ${STATUS_JSON}"
  exit 1
fi

echo "[smoke] PASS"
