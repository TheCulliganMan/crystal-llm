from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from pyboy import PyBoy


EXPECTED_ROM_SHA1 = "f4cd194bdee0d04ca4eac29e09b8e4e9d818c133"

# WRAM/HRAM symbols from the pinned pokecrystal.sym.  These are semantic
# checkpoints, not a cycle-accurate memory contract.
ADDR = {
    "game_time_hours": 0xD4C4,
    "game_time_minutes": 0xD4C6,
    "game_time_seconds": 0xD4C7,
    "game_time_frames": 0xD4C8,
    "money": 0xD84E,
    "map_group": 0xDCB5,
    "map_number": 0xDCB6,
    "y": 0xDCB7,
    "x": 0xDCB8,
    "party_count": 0xDCD7,
    "random_add": 0xFFE1,
    "random_sub": 0xFFE2,
}


def snapshot(pyboy: PyBoy, frame: int) -> dict[str, Any]:
    return {
        "frame": frame,
        "map_group": pyboy.memory[ADDR["map_group"]],
        "map_number": pyboy.memory[ADDR["map_number"]],
        "tile": {
            "x": pyboy.memory[ADDR["x"]],
            "y": pyboy.memory[ADDR["y"]],
        },
        "party_count": pyboy.memory[ADDR["party_count"]],
        "money_bcd": bytes(pyboy.memory[ADDR["money"] : ADDR["money"] + 3]).hex(),
        "rtc": {
            "hours": pyboy.memory[ADDR["game_time_hours"]],
            "minutes": pyboy.memory[ADDR["game_time_minutes"]],
            "seconds": pyboy.memory[ADDR["game_time_seconds"]],
            "frames": pyboy.memory[ADDR["game_time_frames"]],
        },
        "rng": {
            "add": pyboy.memory[ADDR["random_add"]],
            "sub": pyboy.memory[ADDR["random_sub"]],
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Emit semantic checkpoints from the ASM ROM")
    parser.add_argument("rom", type=Path)
    parser.add_argument("--scenario", type=Path, help="JSON action scenario")
    parser.add_argument("--frames", type=int, default=120)
    args = parser.parse_args()

    digest = hashlib.sha1(args.rom.read_bytes()).hexdigest()
    if digest != EXPECTED_ROM_SHA1:
        raise SystemExit(f"unexpected ROM SHA-1: expected {EXPECTED_ROM_SHA1}, found {digest}")

    scenario = json.loads(args.scenario.read_text()) if args.scenario else {}
    actions = scenario.get("actions", [])
    frame_count = max(args.frames, len(actions))
    pyboy = PyBoy(str(args.rom), window="null")
    try:
        trace = [snapshot(pyboy, 0)]
        for frame in range(frame_count):
            buttons = actions[frame].get("buttons", []) if frame < len(actions) else []
            for button in buttons:
                pyboy.button_press(button)
            pyboy.tick()
            for button in buttons:
                pyboy.button_release(button)
            trace.append(snapshot(pyboy, frame + 1))
        print(json.dumps({"rom_sha1": digest, "snapshots": trace}, sort_keys=True))
    finally:
        pyboy.stop()


if __name__ == "__main__":
    main()
