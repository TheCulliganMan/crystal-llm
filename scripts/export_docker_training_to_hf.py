#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "datasets>=2.19.0",
#   "huggingface_hub>=0.23.0",
# ]
# ///
"""Export Docker-recorded PokeCrystal training traces to HF-style JSONL."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from datasets import DatasetDict, load_dataset
from huggingface_hub import HfApi


DEFAULT_INPUT_DIR = Path(".pokecrystal-docker-data/training/web/.pokecrystal-agents/runs/codex-service/training")
DEFAULT_OUTPUT_DIR = Path("datasets/pokecrystal-codex-service-hf")

SYSTEM_PROMPT = " ".join(
    [
        "You are playing Pokemon Crystal through an MCP game interface.",
        "Choose exactly one honest Game Boy-faithful input that advances the current game state.",
        "Do not wait or idle; this runtime resolves inputs instantly.",
        "Return only JSON with the chosen action.",
    ]
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-dir", type=Path, default=DEFAULT_INPUT_DIR)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT_DIR)
    parser.add_argument(
        "--exclude-negative",
        action="store_true",
        help="Skip blocked, busy, no-effect, and tool-error rows instead of exporting them with low weight.",
    )
    parser.add_argument(
        "--segment-length",
        type=int,
        default=6,
        help="Maximum number of contiguous clean actions to include in segment-planning examples.",
    )
    parser.add_argument(
        "--segment-stride",
        type=int,
        default=3,
        help="Step size between segment-planning windows.",
    )
    parser.add_argument(
        "--no-segments",
        action="store_true",
        help="Only export single-turn SFT rows.",
    )
    parser.add_argument(
        "--validate-with-hf",
        action="store_true",
        help="Deprecated: Hugging Face datasets validation now always runs.",
    )
    parser.add_argument(
        "--push-to-hub",
        metavar="REPO_ID",
        help="Push the generated dataset to a Hugging Face dataset repo, for example user/pokecrystal.",
    )
    parser.add_argument(
        "--private",
        action="store_true",
        help="Create/use a private Hugging Face Hub dataset repo when --push-to-hub is set.",
    )
    return parser.parse_args()


def list_episode_files(input_dir: Path) -> list[Path]:
    files = [
        path
        for path in input_dir.iterdir()
        if path.name == "episode.jsonl" or path.name.startswith("episode.jsonl.oversized-")
    ]
    return sorted(files, key=lambda path: path.stat().st_mtime)


def parse_compact_string(text: str, key: str) -> str | None:
    match = re.search(rf"^\s*{re.escape(key)}:\s*(.+)$", text, re.MULTILINE)
    return match.group(1).strip() if match else None


def parse_action_result(text: str) -> dict[str, Any]:
    return {
        "ok": parse_compact_string(text, "ok") == "1",
        "changed": parse_compact_string(text, "ch") == "1",
        "effect": parse_compact_string(text, "fx"),
        "reason": parse_compact_string(text, "rsn"),
        "event": parse_compact_string(text, "ev"),
        "raw": text,
    }


def normalize_action(decision: dict[str, Any] | None) -> dict[str, Any]:
    decision = decision or {}
    action_type = decision.get("actionType")
    if action_type == "move":
        return {"type": "move", "direction": decision.get("direction")}
    if action_type == "press":
        return {"type": "press", "button": decision.get("button")}
    if action_type == "wait":
        return {"type": "wait", "frames": decision.get("frames")}
    return {"type": "unknown"}


def quality_label(row: dict[str, Any], result: dict[str, Any]) -> str:
    tags = row.get("tags") or {}
    if tags.get("toolError") or result["ok"] is False:
        return "negative"
    if result["effect"] in {"blocked", "no_effect", "busy", "menu_locked"}:
        return "negative"
    if result["reason"] in {"blocked", "busy"}:
        return "negative"
    if tags.get("changed") is True or result["changed"] is True:
        mode = (row.get("status") or {}).get("mode")
        if mode in {"battle", "menu"}:
            return "positive"
        if result["effect"] in {"advanced_dialogue", "opened_prompt"}:
            return "positive"
        return "neutral"
    return "negative"


def build_user_content(row: dict[str, Any]) -> str:
    return "\n".join(
        [
            "Current objective:",
            row.get("current_objective") or "Continue making honest main-story progress in Pokemon Crystal.",
            "",
            "Current subgoal:",
            row.get("current_subgoal") or "Choose the next Game Boy-faithful input.",
            "",
            "Observed state:",
            row.get("observer_text") or "",
            "",
            "Status:",
            row.get("status_raw") or (row.get("status") or {}).get("raw") or "",
            "",
            "Recent events:",
            row.get("recent_events_raw") or (row.get("recent_events") or {}).get("raw") or "",
            "",
            "Choose the next input.",
        ]
    )


def build_segment_user_content(rows: list[dict[str, Any]]) -> str:
    first = rows[0]
    return "\n".join(
        [
            "Current objective:",
            first.get("current_objective") or "Continue making honest main-story progress in Pokemon Crystal.",
            "",
            "Current subgoal:",
            "Choose a short sequence of concrete Game Boy-faithful inputs and explain why it helps.",
            "",
            "Observed state:",
            first.get("observer_text") or "",
            "",
            "Status:",
            first.get("status_raw") or (first.get("status") or {}).get("raw") or "",
            "",
            "Recent events:",
            first.get("recent_events_raw") or (first.get("recent_events") or {}).get("raw") or "",
            "",
            "Return JSON with a concise rationale and an actions array.",
        ]
    )


def split_for_id(record_id: str) -> str:
    first_byte = hashlib.sha256(record_id.encode("utf-8")).digest()[0]
    if first_byte < 3:
        return "test"
    if first_byte < 6:
        return "validation"
    return "train"


def action_key(action: dict[str, Any]) -> str:
    if action["type"] == "move":
        return f"move:{action.get('direction')}"
    if action["type"] == "press":
        return f"press:{action.get('button')}"
    return str(action["type"])


def row_identity(row: dict[str, Any]) -> tuple[str | None, tuple[Any, ...] | None, str | None]:
    status = row.get("status") or {}
    coords = status.get("coords")
    return status.get("mode"), tuple(coords) if isinstance(coords, list) else None, status.get("map")


def infer_segment_rationale(rows: list[dict[str, Any]], results: list[dict[str, Any]]) -> str:
    start_status = rows[0].get("status") or {}
    end_status = rows[-1].get("status") or {}
    start_map = start_status.get("map")
    end_map = end_status.get("map")
    start_mode = start_status.get("mode")
    end_mode = end_status.get("mode")
    effects = {result.get("effect") for result in results}
    actions = [normalize_action(row.get("decision")) for row in rows]

    if start_mode != end_mode and end_mode:
        return f"Advance from {start_mode or 'the current mode'} into {end_mode} so the next decision can handle the new game state."
    if start_map != end_map and end_map:
        return f"Move from {start_map or 'the current area'} toward {end_map}, preserving route progress instead of dithering."
    if "opened_prompt" in effects:
        return "Advance dialogue until the game reaches a prompt that needs an explicit answer."
    if "advanced_dialogue" in effects:
        return "Clear dialogue or battle text so control returns to the next actionable state."
    if any((row.get("status") or {}).get("mode") == "battle" for row in rows):
        return "Navigate the battle/menu state with valid button presses so the fight keeps progressing."
    if any((row.get("status") or {}).get("mode") == "menu" for row in rows):
        return "Navigate the menu with valid inputs to reach the next useful game state."
    if actions and all(action.get("type") == "move" for action in actions):
        return "Follow the visible walkable route toward the next transition or interaction point."
    return "Make a short sequence of valid inputs that changes state and keeps the run moving forward."


def segment_is_clean(rows: list[dict[str, Any]]) -> bool:
    if len(rows) < 3:
        return False
    previous_step = None
    previous_identity = None
    stagnant_count = 0
    for row in rows:
        step = row.get("step_index")
        if previous_step is not None and isinstance(step, int) and step != previous_step + 1:
            return False
        result = parse_action_result(row.get("action_result") or "")
        if quality_label(row, result) == "negative":
            return False
        if result.get("changed") is not True:
            return False
        identity = row_identity(row)
        if identity == previous_identity:
            stagnant_count += 1
            if stagnant_count >= 3:
                return False
        else:
            stagnant_count = 0
        previous_identity = identity
        previous_step = step if isinstance(step, int) else previous_step
    return True


def create_segment_record(
    rows: list[dict[str, Any]], source_file: Path, source_line: int, repo_root: Path
) -> dict[str, Any]:
    actions = [normalize_action(row.get("decision")) for row in rows]
    results = [parse_action_result(row.get("action_result") or "") for row in rows]
    labels = [quality_label(row, result) for row, result in zip(rows, results)]
    first = rows[0]
    last = rows[-1]
    status = first.get("status") or {}
    record_id = (
        f"segment:{first.get('session_id', 'unknown')}:{source_file.name}:"
        f"{first.get('step_index') or source_line}-{last.get('step_index') or source_line + len(rows) - 1}"
    )
    return {
        "id": record_id,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": build_segment_user_content(rows)},
            {
                "role": "assistant",
                "content": json.dumps(
                    {"rationale": infer_segment_rationale(rows, results), "actions": actions},
                    separators=(",", ":"),
                ),
            },
        ],
        "actions": actions,
        "quality_label": "positive" if "positive" in labels else "neutral",
        "weight": 1.0 if "positive" in labels else 0.85,
        "segment": {
            "length": len(rows),
            "start_step": first.get("step_index"),
            "end_step": last.get("step_index"),
            "effects": [result.get("effect") for result in results],
            "start_state": {
                "mode": status.get("mode"),
                "map": status.get("map"),
                "coords": status.get("coords"),
                "facing": status.get("facing"),
            },
            "end_state": {
                "mode": (last.get("status") or {}).get("mode"),
                "map": (last.get("status") or {}).get("map"),
                "coords": (last.get("status") or {}).get("coords"),
                "facing": (last.get("status") or {}).get("facing"),
            },
        },
        "provenance": {
            "source_file": str(source_file.relative_to(repo_root)),
            "source_line": source_line,
            "session_id": first.get("session_id"),
            "recorded_at": first.get("recorded_at"),
            "model": first.get("model"),
        },
    }


def create_record(row: dict[str, Any], source_file: Path, source_line: int, repo_root: Path) -> dict[str, Any]:
    action = normalize_action(row.get("decision"))
    result = parse_action_result(row.get("action_result") or "")
    label = quality_label(row, result)
    record_id = f"{row.get('session_id', 'unknown')}:{source_file.name}:{row.get('step_index') or source_line}"
    status = row.get("status") or {}
    return {
        "id": record_id,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": build_user_content(row)},
            {"role": "assistant", "content": json.dumps(action, separators=(",", ":"))},
        ],
        "action": action,
        "quality_label": label,
        "weight": 1.0 if label == "positive" else 0.75 if label == "neutral" else 0.25,
        "outcome": result,
        "state": {
            "mode": status.get("mode"),
            "map": status.get("map"),
            "coords": status.get("coords"),
            "facing": status.get("facing"),
            "inBattle": status.get("inBattle"),
            "inMenu": status.get("inMenu"),
            "inDialog": status.get("inDialog"),
            "canMove": status.get("canMove"),
        },
        "provenance": {
            "source_file": str(source_file.relative_to(repo_root)),
            "source_line": source_line,
            "session_id": row.get("session_id"),
            "step_index": row.get("step_index"),
            "recorded_at": row.get("recorded_at"),
            "model": row.get("model"),
        },
    }


def write_dataset_card(output_dir: Path, manifest: dict[str, Any]) -> None:
    body = f"""---
configs:
  - config_name: default
    data_files:
      - split: train
        path: train.jsonl
      - split: validation
        path: validation.jsonl
      - split: test
        path: test.jsonl
  - config_name: segments
    data_files:
      - split: train
        path: segment_train.jsonl
      - split: validation
        path: segment_validation.jsonl
      - split: test
        path: segment_test.jsonl
---

# PokeCrystal Docker Gameplay SFT Dataset

This dataset was exported from Docker-recorded MCP gameplay traces in `{manifest["input_dir"]}`.

Each row is a chat-style supervised fine-tuning example:

- `messages[0]`: system instruction for instant Pokemon Crystal input selection.
- `messages[1]`: observed map/state/status/recent events.
- `messages[2]`: assistant JSON action.
- `quality_label`: heuristic `positive`, `neutral`, or `negative`.
- `weight`: suggested training weight.
- `outcome`, `state`, and `provenance`: filtering/debug metadata.

The `segments` config groups clean contiguous action windows into higher-level
planning examples. Segment assistant messages return JSON with:

- `rationale`: an inferred reason the sequence helps.
- `actions`: the short input sequence to execute.

Export summary:

```json
{json.dumps(manifest["counts"], indent=2)}
```
"""
    (output_dir / "README.md").write_text(body, encoding="utf-8")


def load_hf_dataset(output_dir: Path) -> Any:
    turns = load_dataset(
        "json",
        data_files={
            "train": str(output_dir / "train.jsonl"),
            "validation": str(output_dir / "validation.jsonl"),
            "test": str(output_dir / "test.jsonl"),
        },
    )
    segments = load_dataset(
        "json",
        data_files={
            "train": str(output_dir / "segment_train.jsonl"),
            "validation": str(output_dir / "segment_validation.jsonl"),
            "test": str(output_dir / "segment_test.jsonl"),
        },
    )
    return DatasetDict(
        {
            "train": turns["train"],
            "validation": turns["validation"],
            "test": turns["test"],
            "segment_train": segments["train"],
            "segment_validation": segments["validation"],
            "segment_test": segments["test"],
        }
    )


def validate_with_hf(output_dir: Path) -> dict[str, int]:
    dataset = load_hf_dataset(output_dir)
    return {split: len(dataset[split]) for split in dataset}


def push_to_hub(output_dir: Path, repo_id: str, private: bool) -> str:
    dataset = load_hf_dataset(output_dir)
    api = HfApi()
    api.create_repo(repo_id=repo_id, repo_type="dataset", private=private, exist_ok=True)
    dataset.push_to_hub(repo_id, private=private)
    api.upload_file(
        path_or_fileobj=str(output_dir / "README.md"),
        path_in_repo="README.md",
        repo_id=repo_id,
        repo_type="dataset",
    )
    api.upload_file(
        path_or_fileobj=str(output_dir / "manifest.json"),
        path_in_repo="manifest.json",
        repo_id=repo_id,
        repo_type="dataset",
    )
    return f"https://huggingface.co/datasets/{repo_id}"


def increment(counter: dict[str, int], key: str) -> None:
    counter[key] = counter.get(key, 0) + 1


def export_dataset(args: argparse.Namespace) -> dict[str, Any]:
    repo_root = Path.cwd().resolve()
    input_dir = args.input_dir.resolve()
    output_dir = args.output_dir.resolve()
    episode_files = list_episode_files(input_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    writers = {
        "train": (output_dir / "train.jsonl").open("w", encoding="utf-8"),
        "validation": (output_dir / "validation.jsonl").open("w", encoding="utf-8"),
        "test": (output_dir / "test.jsonl").open("w", encoding="utf-8"),
    }
    segment_writers = {
        "train": (output_dir / "segment_train.jsonl").open("w", encoding="utf-8"),
        "validation": (output_dir / "segment_validation.jsonl").open("w", encoding="utf-8"),
        "test": (output_dir / "segment_test.jsonl").open("w", encoding="utf-8"),
    }
    counts: dict[str, Any] = {
        "files": len(episode_files),
        "source_rows": 0,
        "exported_rows": 0,
        "malformed_rows": 0,
        "skipped_negative_rows": 0,
        "splits": {"train": 0, "validation": 0, "test": 0},
        "labels": {"positive": 0, "neutral": 0, "negative": 0},
        "modes": {},
        "maps": {},
        "actions": {},
        "segments": {
            "exported_rows": 0,
            "splits": {"train": 0, "validation": 0, "test": 0},
            "labels": {"positive": 0, "neutral": 0},
            "lengths": {},
        },
    }
    malformed: list[dict[str, Any]] = []

    try:
        for episode_file in episode_files:
            rows_for_segments: list[tuple[int, dict[str, Any]]] = []
            with episode_file.open("r", encoding="utf-8") as source:
                for source_line, line in enumerate(source, start=1):
                    if not line.strip():
                        continue
                    counts["source_rows"] += 1
                    try:
                        row = json.loads(line)
                    except json.JSONDecodeError as error:
                        counts["malformed_rows"] += 1
                        if len(malformed) < 20:
                            malformed.append(
                                {
                                    "source_file": str(episode_file.relative_to(repo_root)),
                                    "source_line": source_line,
                                    "error": str(error),
                                }
                            )
                        continue

                    rows_for_segments.append((source_line, row))
                    record = create_record(row, episode_file, source_line, repo_root)
                    if args.exclude_negative and record["quality_label"] == "negative":
                        counts["skipped_negative_rows"] += 1
                        continue

                    split = split_for_id(record["id"])
                    writers[split].write(json.dumps(record, separators=(",", ":")) + "\n")
                    counts["exported_rows"] += 1
                    counts["splits"][split] += 1
                    counts["labels"][record["quality_label"]] += 1
                    increment(counts["modes"], record["state"].get("mode") or "unknown")
                    increment(counts["maps"], record["state"].get("map") or "unknown")
                    increment(counts["actions"], action_key(record["action"]))

            if not args.no_segments:
                max_length = max(3, args.segment_length)
                stride = max(1, args.segment_stride)
                for start_index in range(0, max(0, len(rows_for_segments) - 2), stride):
                    window = rows_for_segments[start_index : start_index + max_length]
                    if len(window) < 3:
                        continue
                    rows = [row for _, row in window]
                    if not segment_is_clean(rows):
                        continue
                    source_line = window[0][0]
                    record = create_segment_record(rows, episode_file, source_line, repo_root)
                    split = split_for_id(record["id"])
                    segment_writers[split].write(json.dumps(record, separators=(",", ":")) + "\n")
                    counts["segments"]["exported_rows"] += 1
                    counts["segments"]["splits"][split] += 1
                    counts["segments"]["labels"][record["quality_label"]] += 1
                    increment(counts["segments"]["lengths"], str(record["segment"]["length"]))
    finally:
        for writer in writers.values():
            writer.close()
        for writer in segment_writers.values():
            writer.close()

    manifest = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "input_dir": str(input_dir.relative_to(repo_root)),
        "output_dir": str(output_dir.relative_to(repo_root)),
        "include_negative": not args.exclude_negative,
        "source_files": [str(path.relative_to(repo_root)) for path in episode_files],
        "counts": counts,
        "malformed": malformed,
    }
    (output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    write_dataset_card(output_dir, manifest)
    return manifest


def main() -> None:
    args = parse_args()
    manifest = export_dataset(args)
    output_dir = args.output_dir.resolve()
    manifest["huggingface_validation"] = validate_with_hf(output_dir)
    (output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    if args.push_to_hub:
        manifest["huggingface_url"] = push_to_hub(output_dir, args.push_to_hub, args.private)
        (output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
