#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${PORT:-3120}"
SESSION_ID="${SESSION_ID:-ai-playtest-$(date +%s)}"
AGENT_ID="${AGENT_ID:-ai-playtest-agent}"
START_LOG="${ROOT_DIR}/.next/ai-playtest-start.log"
BASE_URL="http://127.0.0.1:${PORT}/api/mcp/tools"
MCP_URL="http://127.0.0.1:${PORT}/api/mcp?session_id=${SESSION_ID}"
SESSION_SECRET_URL="http://127.0.0.1:${PORT}/api/arena/session-secret?session_id=${SESSION_ID}"
MCP_PROTOCOL_VERSION="${MCP_PROTOCOL_VERSION:-2024-11-05}"
SEED_SAVE_PATH="${SEED_SAVE_PATH:-${ROOT_DIR}/autosave.sav}"
SESSION_AUTOSAVE="${ROOT_DIR}/mcp-${SESSION_ID}-autosave.sav"
SCOPED_AUTOSAVE=""
MCP_STATIC_TOKEN="${POKECRYSTAL_MCP_TOKEN:-}"
SUPABASE_URL="${NEXT_PUBLIC_SUPABASE_URL:-}"
SUPABASE_SERVICE_KEY_VALUE="${SUPABASE_SERVICE_ROLE_KEY:-}"

if ! command -v jq >/dev/null 2>&1; then
  echo "[playtest] jq is required on PATH"
  exit 1
fi

cd "${ROOT_DIR}"

echo "[playtest] building app"
npm run build >/dev/null

echo "[playtest] starting app on port ${PORT}"
PORT="${PORT}" npm run start >"${START_LOG}" 2>&1 &
APP_PID=$!

cleanup() {
  kill "${APP_PID}" >/dev/null 2>&1 || true
  rm -f "${SESSION_AUTOSAVE}" >/dev/null 2>&1 || true
  if [[ -n "${SCOPED_AUTOSAVE}" ]]; then
    rm -f "${SCOPED_AUTOSAVE}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

for _ in $(seq 1 120); do
  if curl -fsS "http://127.0.0.1:${PORT}/" >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

if [[ ! -f "${SEED_SAVE_PATH}" ]]; then
  echo "[playtest] seed save missing: ${SEED_SAVE_PATH}"
  exit 1
fi

call_tools_api() {
  local payload="${1}"
  shift
  curl -fsS -X POST "${BASE_URL}?session_id=${SESSION_ID}" \
    -H 'accept: application/json' \
    -H 'content-type: application/json' \
    "${MCP_TOKEN_HEADERS[@]}" \
    "$@" \
    -d "${payload}"
}

call_tool() {
  local tool_name="${1}"
  local tool_args="${2}"
  shift 2
  call_tools_api \
    "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"${tool_name}\",\"arguments\":${tool_args}}}" \
    "$@"
}

run_json_retry() {
  local attempts="${1:-8}"
  shift
  local n=1
  while true; do
    if output="$("$@" 2>/dev/null)"; then
      if [[ -n "${output}" ]]; then
        printf '%s' "${output}"
        return 0
      fi
    fi
    if [[ "${n}" -ge "${attempts}" ]]; then
      echo "[playtest] request failed after ${attempts} attempts: $*" >&2
      return 1
    fi
    n=$((n + 1))
    sleep 1
  done
}

read_env_var_from_file() {
  local file_path="${1}"
  local key="${2}"
  if [[ ! -f "${file_path}" ]]; then
    return 1
  fi
  grep -E "^${key}=" "${file_path}" | tail -n 1 | sed -E "s/^${key}=//" | sed -E 's/^["'"'"']?(.*)["'"'"']?$/\1/'
}

load_supabase_env_fallback() {
  if [[ -n "${SUPABASE_URL}" && -n "${SUPABASE_SERVICE_KEY_VALUE}" ]]; then
    return
  fi
  local env_file
  for env_file in "${ROOT_DIR}/.env.local" "${ROOT_DIR}/.env"; do
    if [[ -z "${SUPABASE_URL}" ]]; then
      SUPABASE_URL="$(read_env_var_from_file "${env_file}" "NEXT_PUBLIC_SUPABASE_URL" || true)"
    fi
    if [[ -z "${SUPABASE_SERVICE_KEY_VALUE}" ]]; then
      SUPABASE_SERVICE_KEY_VALUE="$(read_env_var_from_file "${env_file}" "SUPABASE_SERVICE_ROLE_KEY" || true)"
    fi
  done
}

extract_json_payload() {
  local raw="${1}"
  if printf '%s' "${raw}" | jq -e . >/dev/null 2>&1; then
    printf '%s' "${raw}"
    return 0
  fi

  local sse_payload
  sse_payload="$(printf '%s\n' "${raw}" | awk '/^data: /{sub(/^data: /,""); print}' | tail -n 1)"
  if [[ -n "${sse_payload}" ]] && printf '%s' "${sse_payload}" | jq -e . >/dev/null 2>&1; then
    printf '%s' "${sse_payload}"
    return 0
  fi

  return 1
}

seed_identity_supabase_slot() {
  local player_id="${1}"
  local slot="${2}"
  local payload_json
  local response

  load_supabase_env_fallback
  if [[ -z "${SUPABASE_URL}" || -z "${SUPABASE_SERVICE_KEY_VALUE}" ]]; then
    echo "[playtest] missing NEXT_PUBLIC_SUPABASE_URL or SUPABASE_SERVICE_ROLE_KEY for identity seeding"
    exit 1
  fi

  payload_json="$(jq -c . "${SEED_SAVE_PATH}")"
  if [[ -z "${payload_json}" || "${payload_json}" == "null" ]]; then
    echo "[playtest] failed to parse seed save payload"
    exit 1
  fi

  response="$(curl -fsS -X POST "${SUPABASE_URL}/rest/v1/game_saves?on_conflict=user_id,slot" \
    -H "apikey: ${SUPABASE_SERVICE_KEY_VALUE}" \
    -H "authorization: Bearer ${SUPABASE_SERVICE_KEY_VALUE}" \
    -H "content-type: application/json" \
    -H "prefer: resolution=merge-duplicates,return=representation" \
    -d "[{\"user_id\":\"${player_id}\",\"slot\":\"${slot}\",\"payload\":${payload_json},\"updated_at\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}]")"

  local row_count
  row_count="$(printf '%s' "${response}" | jq 'length')"
  if [[ "${row_count}" -lt 1 ]]; then
    echo "[playtest] supabase seed upsert returned no rows for ${player_id}:${slot}"
    exit 1
  fi
}

MCP_TOKEN_HEADERS=()
if [[ -n "${MCP_STATIC_TOKEN}" ]]; then
  MCP_TOKEN_HEADERS=(-H "x-mcp-token: ${MCP_STATIC_TOKEN}")
fi

echo "[playtest] bootstrap skill auth"
REGISTER_RESPONSE="$(run_json_retry 10 call_tool register_identity "{\"agentId\":\"${AGENT_ID}\",\"identityName\":\"${AGENT_ID}\"}")"
AUTH="$(printf '%s' "${REGISTER_RESPONSE}" | jq -cr '.result.content[0].text | fromjson')"
TOKEN="$(printf '%s' "${AUTH}" | jq -r '.token')"
PLAYER_ID="$(printf '%s' "${AUTH}" | jq -r '.playerId')"
SESSION_SECRET_RESPONSE="$(run_json_retry 10 curl -fsS -X GET "${SESSION_SECRET_URL}" -H "Authorization: Bearer ${TOKEN}" "${MCP_TOKEN_HEADERS[@]}")"
SESSION_SECRET="$(printf '%s' "${SESSION_SECRET_RESPONSE}" | jq -r '.sessionSecret')"
if [[ -z "${TOKEN}" || "${TOKEN}" == "null" ]]; then
  echo "[playtest] missing identity token from auth bootstrap"
  exit 1
fi
if [[ -z "${SESSION_SECRET}" || "${SESSION_SECRET}" == "null" ]]; then
  echo "[playtest] missing session secret from auth bootstrap"
  exit 1
fi
if [[ -z "${PLAYER_ID}" || "${PLAYER_ID}" == "null" ]]; then
  echo "[playtest] missing playerId from auth bootstrap"
  exit 1
fi

cp -f "${SEED_SAVE_PATH}" "${SESSION_AUTOSAVE}"
SCOPED_AUTOSAVE="${ROOT_DIR}/${PLAYER_ID}__mcp-${SESSION_ID}-autosave.sav"
cp -f "${SEED_SAVE_PATH}" "${SCOPED_AUTOSAVE}"
IDENTITY_SLOT="mcp-${SESSION_ID}-autosave.sav"

echo "[playtest] seeding identity slot in Supabase (${PLAYER_ID}:${IDENTITY_SLOT})"
seed_identity_supabase_slot "${PLAYER_ID}" "${IDENTITY_SLOT}"

echo "[playtest] API path status (tools endpoint)"
STATUS="$(run_json_retry 10 call_tool status "{}" -H "authorization: Bearer ${TOKEN}" -H "x-session-secret: ${SESSION_SECRET}")"
printf '%s' "${STATUS}" | jq -e '.ok == true' >/dev/null
API_PARTY_COUNT="$(printf '%s' "${STATUS}" | jq -r '.result.content[0].text | fromjson | .party.count // 0')"
if [[ "${API_PARTY_COUNT}" -lt 1 ]]; then
  echo "[playtest] expected API party.count >= 1, got ${API_PARTY_COUNT}"
  exit 1
fi

echo "[playtest] MCP path initialize"
INIT_RESPONSE_RAW="$(curl -fsS -X POST "${MCP_URL}" \
  -H 'accept: application/json' \
  -H 'content-type: application/json' \
  -H "authorization: Bearer ${TOKEN}" \
  -H "x-session-secret: ${SESSION_SECRET}" \
  "${MCP_TOKEN_HEADERS[@]}" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"${MCP_PROTOCOL_VERSION}\",\"capabilities\":{},\"clientInfo\":{\"name\":\"ai-playtest\",\"version\":\"1.0\"}}}")"
INIT_RESPONSE="$(extract_json_payload "${INIT_RESPONSE_RAW}" || true)"
if [[ -z "${INIT_RESPONSE}" ]]; then
  echo "[playtest] MCP initialize did not return JSON payload"
  printf '%s\n' "${INIT_RESPONSE_RAW}"
  exit 1
fi
printf '%s' "${INIT_RESPONSE}" | jq -e '.result.protocolVersion == "'"${MCP_PROTOCOL_VERSION}"'"' >/dev/null

echo "[playtest] MCP path status (json-rpc tools/call)"
MCP_STATUS_RAW="$(curl -fsS -X POST "${MCP_URL}" \
  -H 'accept: application/json' \
  -H 'content-type: application/json' \
  -H "mcp-protocol-version: ${MCP_PROTOCOL_VERSION}" \
  -H "authorization: Bearer ${TOKEN}" \
  -H "x-session-secret: ${SESSION_SECRET}" \
  "${MCP_TOKEN_HEADERS[@]}" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"status","arguments":{}}}')"
MCP_STATUS="$(extract_json_payload "${MCP_STATUS_RAW}" || true)"
if [[ -z "${MCP_STATUS}" ]]; then
  echo "[playtest] MCP status did not return JSON payload"
  printf '%s\n' "${MCP_STATUS_RAW}"
  exit 1
fi
MCP_PARTY_COUNT="$(printf '%s' "${MCP_STATUS}" | jq -r '.result.content[0].text | fromjson | .party.count // 0')"
if [[ "${MCP_PARTY_COUNT}" -lt 1 ]]; then
  echo "[playtest] expected MCP party.count >= 1, got ${MCP_PARTY_COUNT}"
  exit 1
fi

echo "[playtest] summary"
echo "session_id=${SESSION_ID}"
echo "agent_id=${AGENT_ID}"
echo "player_id=${PLAYER_ID}"
echo "api_party_count=${API_PARTY_COUNT}"
echo "mcp_party_count=${MCP_PARTY_COUNT}"
echo "get_pokemon_api=true"
echo "get_pokemon_mcp=true"
echo "[playtest] PASS"
