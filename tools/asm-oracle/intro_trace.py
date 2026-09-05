from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any

from pyboy import PyBoy


EXPECTED_ROM_SHA1 = "f4cd194bdee0d04ca4eac29e09b8e4e9d818c133"
JUMPTABLE_EXIT = 1 << 7
EXPECTED_SCENE_TRANSITION_FRAMES = (
    1,
    66,
    195,
    240,
    369,
    435,
    564,
    667,
    763,
    770,
    963,
    1012,
    1205,
    1288,
    1418,
    1485,
    1634,
    1700,
    1797,
    1868,
    2022,
    2026,
    2035,
    2036,
    2069,
    2133,
    2180,
    2309,
)
EXPECTED_COMPLETION_FRAME = 2442
EXPECTED_OUTER_LOOP_ITERATIONS = 1752
EXPECTED_OUTER_LOOP_FRAME_BOUNDARIES = 2441
EXPECTED_ENTRY_TO_FIRST_INPUT_MACHINE_CYCLES = 59
EXPECTED_JOY_TEXT_DELAY_CALLS = EXPECTED_OUTER_LOOP_ITERATIONS + 1
EXPECTED_JOY_TEXT_DELAY_TIMER_CALLS = 5
EXPECTED_TIMER_INTERRUPT_PERIOD_T_CYCLES = 262_144
EXPECTED_FIRST_TIMER_REQUEST_AFTER_INTRO_ENTRY_T_CYCLES = 258_428
EXPECTED_TIMER_INTERRUPT_COUNT = 654
EXPECTED_LCD_HBLANK_REQUEST_T_CYCLES = 250
EXPECTED_INTRO_VBLANK_COUNT = EXPECTED_COMPLETION_FRAME - 1
EXPECTED_JOY_TEXT_DELAY_BODY_COUNTS = {107: 1395, 110: 358}
JOY_TEXT_DELAY_COMMON_INSTRUCTION_MACHINE_CYCLES = (
    6, 4, 4, 4, 4, 4, 2, 2, 3, 1, 3, 1, 1, 1, 1, 3, 1, 1,
    3, 1, 1, 3, 3, 3, 3, 3, 4, 3, 1, 3, 2, 3, 3, 3, 1,
)
JOY_TEXT_DELAY_TAIL_MACHINE_CYCLES = {
    107: (3, 4, 1, 2, 1, 3, 4),
    110: (3, 4, 1, 3, 2, 4, 4),
}
EXPECTED_VBLANK_NON_AUDIO_T_CYCLES_TO_GAME_TIMER = 2356
EXPECTED_INACTIVE_CHANNEL_SOUND_UPDATE_T_CYCLES = 1364


def load_symbols(path: Path) -> dict[str, tuple[int, int]]:
    symbols: dict[str, tuple[int, int]] = {}
    pattern = re.compile(r"^([0-9a-fA-F]{2}):([0-9a-fA-F]{4})\s+(\S+)$")
    for raw_line in path.read_text().splitlines():
        match = pattern.match(raw_line)
        if match:
            symbols[match.group(3)] = (
                int(match.group(1), 16),
                int(match.group(2), 16),
            )
    return symbols


def symbol_address(symbols: dict[str, tuple[int, int]], name: str) -> int:
    try:
        return symbols[name][1]
    except KeyError as error:
        raise SystemExit(f"required symbol is missing: {name}") from error


def read_vram(pyboy: PyBoy, bank: int, start: int, size: int) -> bytes:
    saved_bank = pyboy.memory[0xFF4F]
    pyboy.memory[0xFF4F] = bank
    result = bytes(pyboy.memory[start : start + size])
    pyboy.memory[0xFF4F] = saved_bank
    return result


def read_wram_bank(pyboy: PyBoy, bank: int, start: int, size: int) -> bytes:
    saved_bank = pyboy.memory[0xFF70]
    pyboy.memory[0xFF70] = bank
    result = bytes(pyboy.memory[start : start + size])
    pyboy.memory[0xFF70] = saved_bank
    return result


def capture(
    pyboy: PyBoy,
    absolute_frame: int,
    intro_frame: int,
    addresses: dict[str, int],
) -> dict[str, Any]:
    pixels = bytes(pyboy.screen.ndarray.tobytes())
    rgb5_pixels = bytes(
        component >> 3
        for offset, component in enumerate(pixels)
        if offset % 4 != 3
    )
    tilemap_9800 = read_vram(pyboy, 0, 0x9800, 32 * 32)
    attrmap_9800 = read_vram(pyboy, 1, 0x9800, 32 * 32)
    tilemap_9c00 = read_vram(pyboy, 0, 0x9C00, 32 * 32)
    attrmap_9c00 = read_vram(pyboy, 1, 0x9C00, 32 * 32)
    palettes = read_wram_bank(pyboy, 5, addresses["bg_pals_1"], 0x100)
    ly_overrides = read_wram_bank(pyboy, 5, addresses["ly_overrides"], 0x100)
    return {
        "absolute_frame": absolute_frame,
        "intro_frame": intro_frame,
        "rgba_sha256": hashlib.sha256(pixels).hexdigest(),
        "rgb5_sha256": hashlib.sha256(rgb5_pixels).hexdigest(),
        "vram_9800_tilemap_sha256": hashlib.sha256(tilemap_9800).hexdigest(),
        "vram_9800_attrmap_sha256": hashlib.sha256(attrmap_9800).hexdigest(),
        "vram_9c00_tilemap_sha256": hashlib.sha256(tilemap_9c00).hexdigest(),
        "vram_9c00_attrmap_sha256": hashlib.sha256(attrmap_9c00).hexdigest(),
        "palette_buffers_sha256": hashlib.sha256(palettes).hexdigest(),
        "ly_overrides_sha256": hashlib.sha256(ly_overrides).hexdigest(),
        "oam_sha256": hashlib.sha256(bytes(pyboy.memory[0xFE00:0xFEA0])).hexdigest(),
        "oam_hex": bytes(pyboy.memory[0xFE00:0xFEA0]).hex(),
        "jumptable_index": pyboy.memory[addresses["jumptable_index"]],
        "scene_frame_counter": pyboy.memory[addresses["scene_frame_counter"]],
        "scene_timer": pyboy.memory[addresses["scene_timer"]],
        "lcdc": pyboy.memory[0xFF40],
        "scy": pyboy.memory[0xFF42],
        "scx": pyboy.memory[0xFF43],
        "wy": pyboy.memory[0xFF4A],
        "wx": pyboy.memory[0xFF4B],
        "h_scx": pyboy.memory[addresses["h_scx"]],
        "h_scy": pyboy.memory[addresses["h_scy"]],
        "h_bg_map_mode": pyboy.memory[addresses["h_bg_map_mode"]],
        "h_bg_map_address": pyboy.memory[addresses["h_bg_map_address"]],
        "h_cgb_pal_update": pyboy.memory[addresses["h_cgb_pal_update"]],
        "h_lcdc_pointer": pyboy.memory[addresses["h_lcdc_pointer"]],
    }


def write_ppm(path: Path, pixels: bytes) -> None:
    rgb = bytes(
        component
        for offset, component in enumerate(pixels)
        if offset % 4 != 3
    )
    path.write_bytes(b"P6\n160 144\n255\n" + rgb)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run the pinned ASM CrystalIntro and emit every scene boundary"
    )
    parser.add_argument("rom", type=Path)
    parser.add_argument("--symbols", type=Path)
    parser.add_argument("--frames", type=int, default=4_000)
    parser.add_argument("--ppm-dir", type=Path, help="write first/last scene frames as PPM")
    parser.add_argument(
        "--timing",
        action="store_true",
        help="record exact ROM CPU clocks crossed by each intro decompression",
    )
    parser.add_argument(
        "--all-vblanks",
        action="store_true",
        help="with --timing, retain every intro VBlank audio/register state",
    )
    args = parser.parse_args()
    if args.all_vblanks and not args.timing:
        parser.error("--all-vblanks requires --timing")

    rom = args.rom.read_bytes()
    digest = hashlib.sha1(rom).hexdigest()
    if digest != EXPECTED_ROM_SHA1:
        raise SystemExit(f"unexpected ROM SHA-1: expected {EXPECTED_ROM_SHA1}, found {digest}")
    symbols = load_symbols(args.symbols or args.rom.with_suffix(".sym"))
    try:
        intro_bank, intro_address = symbols["CrystalIntro"]
    except KeyError as error:
        raise SystemExit("required symbol is missing: CrystalIntro") from error
    addresses = {
        name: symbol_address(symbols, symbol)
        for name, symbol in {
            "jumptable_index": "wJumptableIndex",
            "scene_frame_counter": "wIntroSceneFrameCounter",
            "scene_timer": "wIntroSceneTimer",
            "bg_pals_1": "wBGPals1",
            "ly_overrides": "wLYOverrides",
            "h_scx": "hSCX",
            "h_scy": "hSCY",
            "h_bg_map_mode": "hBGMapMode",
            "h_bg_map_address": "hBGMapAddress",
            "h_cgb_pal_update": "hCGBPalUpdate",
            "h_lcdc_pointer": "hLCDCPointer",
            "h_vblank": "hVBlank",
            "h_oam_update": "hOAMUpdate",
            "requested_2bpp_size": "wRequested2bppSize",
            "requested_1bpp_size": "wRequested1bppSize",
            "music_playing": "wMusicPlaying",
            "audio": "wAudio",
            "audio_end": "wAudioEnd",
            "music_id": "wMusicID",
            "cur_sfx": "wCurSFX",
            "cur_channel": "wCurChannel",
            "cur_music_byte": "wCurMusicByte",
            "channel1": "wChannel1",
            "channel2": "wChannel2",
            "channel1_note_length": "wChannel1NoteLength",
            "channel1_octave": "wChannel1Octave",
            "channel1_transposition": "wChannel1Transposition",
            "channel1_duration_modifier": "wChannel1NoteDurationModifier",
            "channel1_tempo": "wChannel1Tempo",
        }.items()
    }

    if args.ppm_dir:
        args.ppm_dir.mkdir(parents=True, exist_ok=True)
    hook_context: dict[str, Any] = {
        "entered": False,
        "absolute_frame": 0,
        "intro_start_absolute_frame": None,
        "frame_start_t_cycles": 0,
        "intro_start_frame_t_cycles": None,
        "intro_entry_t_cycles": None,
        "active_decompression": None,
        "active_vblank_handler": None,
        "active_sound_update": None,
        "active_music_parse": None,
        "active_sound_helpers": {},
        "sound_helper_hooks_registered": False,
        "active_intro_routine_calls": [],
        "registered_intro_routine_returns": set(),
        "interrupt_counts": {
            "vblank": 0,
            "lcd": 0,
            "timer": 0,
            "serial": 0,
            "joypad": 0,
        },
    }

    def intro_hook(context: dict[str, Any]) -> None:
        context["entered"] = True
        context["intro_start_absolute_frame"] = context["absolute_frame"]
        context["intro_start_frame_t_cycles"] = context["frame_start_t_cycles"]
        context["intro_entry_t_cycles"] = pyboy._cycles()

    checkpoints: list[dict[str, Any]] = []
    transitions: list[dict[str, int]] = []
    decompressions: list[dict[str, Any]] = []
    intro_routine_samples: list[dict[str, Any]] = []
    vblank_handler_samples: list[dict[str, Any]] = []
    timer_interrupt_samples: list[dict[str, Any]] = []
    pyboy = PyBoy(str(args.rom), window="null", log_level="ERROR")
    pyboy.hook_register(intro_bank, intro_address, intro_hook, hook_context)
    if args.timing:
        reverse_symbols: dict[tuple[int, int], list[str]] = {}
        for name, location in symbols.items():
            reverse_symbols.setdefault(location, []).append(name)
        h_rom_bank = symbol_address(symbols, "hROMBank")
        decompress_bank, decompress_address = symbols["Decompress"]

        def current_intro_frame(context: dict[str, Any]) -> int:
            start = context["intro_start_absolute_frame"]
            if not isinstance(start, int):
                return 0
            return int(context["absolute_frame"]) - start + 1

        def decompress_hook(context: dict[str, Any]) -> None:
            if not context["entered"]:
                return
            if context["active_decompression"] is not None:
                raise RuntimeError("nested Decompress call in CrystalIntro")
            source_address = pyboy.register_file.HL
            source_bank = 0 if source_address < 0x4000 else pyboy.memory[h_rom_bank]
            candidates = reverse_symbols.get((source_bank, source_address), [])
            resource_symbols = [
                name
                for name in candidates
                if name.startswith("Intro") or name.startswith("Title")
            ]
            if len(resource_symbols) != 1:
                raise RuntimeError(
                    "intro Decompress source has no unique source symbol: "
                    f"{source_bank:02x}:{source_address:04x} {candidates}"
                )
            context["active_decompression"] = {
                "resource_symbol": resource_symbols[0],
                "source_bank": source_bank,
                "source_address": source_address,
                "start_absolute_frame": context["absolute_frame"],
                "start_intro_frame": current_intro_frame(context),
                "start_t_cycles": pyboy._cycles(),
                "start_interrupt_counts": context["interrupt_counts"].copy(),
            }
            set_sound_helper_hooks(context, True)

        def decompression_return_hook(
            context: dict[str, Any], helper: str
        ) -> None:
            active = context["active_decompression"]
            if active is None:
                return
            end_intro_frame = current_intro_frame(context)
            decompressions.append(
                {
                    **active,
                    "helper": helper,
                    "end_absolute_frame": context["absolute_frame"],
                    "end_intro_frame": end_intro_frame,
                    "elapsed_t_cycles_between_hooks": pyboy._cycles()
                    - active["start_t_cycles"],
                    "frame_boundaries_crossed": end_intro_frame
                    - active["start_intro_frame"],
                    "interrupts": {
                        name: count - active["start_interrupt_counts"][name]
                        for name, count in context["interrupt_counts"].items()
                    },
                }
            )
            context["active_decompression"] = None
            set_sound_helper_hooks(context, False)

        pyboy.hook_register(
            decompress_bank, decompress_address, decompress_hook, hook_context
        )
        for interrupt_name, symbol in {
            "vblank": "VBlank",
            "lcd": "LCD",
            "timer": "MobileTimer",
            "serial": "Serial",
            "joypad": "Joypad",
        }.items():
            interrupt_bank, interrupt_address = symbols[symbol]

            def interrupt_hook(
                context: dict[str, Any], name: str = interrupt_name
            ) -> None:
                context["interrupt_counts"][name] += 1
                if name == "timer" and context["entered"]:
                    timer_interrupt_samples.append(
                        {
                            "intro_frame": current_intro_frame(context),
                            "start_t_cycles": pyboy._cycles(),
                            "div": pyboy.memory[0xFF04],
                            "tima": pyboy.memory[0xFF05],
                            "tma": pyboy.memory[0xFF06],
                            "tac": pyboy.memory[0xFF07],
                            "interrupt_flags": pyboy.memory[0xFF0F],
                            "interrupt_enable": pyboy.memory[0xFFFF],
                        }
                    )
                if name in ("lcd", "timer"):
                    for active_call in reversed(
                        context["active_intro_routine_calls"]
                    ):
                        if active_call["routine"] == "JoyTextDelay":
                            active_call.setdefault("interrupt_events", []).append(
                                {
                                    "interrupt": name,
                                    "start_t_cycles": pyboy._cycles(),
                                    "ly": pyboy.memory[0xFF44],
                                    "lcdc": pyboy.memory[0xFF40],
                                    "scx": pyboy.memory[0xFF43],
                                    "scy": pyboy.memory[0xFF42],
                                    "wx": pyboy.memory[0xFF4B],
                                    "wy": pyboy.memory[0xFF4A],
                                    "lcd_callback_pointer": pyboy.memory[
                                        addresses["h_lcdc_pointer"]
                                    ],
                                    "visible_oam_sha256": hashlib.sha256(
                                        bytes(pyboy.memory[0xFE00:0xFEA0])
                                    ).hexdigest(),
                                }
                            )
                            break
                if (
                    name == "vblank"
                    and context["entered"]
                    and (
                        args.all_vblanks
                        or context["active_decompression"] is not None
                    )
                ):
                    audio_state = bytes(
                        pyboy.memory[addresses["audio"] : addresses["audio_end"]]
                    )
                    active_decompression = context["active_decompression"]
                    context["active_vblank_handler"] = {
                        "resource_symbol": (
                            active_decompression["resource_symbol"]
                            if active_decompression is not None
                            else None
                        ),
                        "intro_frame": current_intro_frame(context),
                        "scene": pyboy.memory[addresses["jumptable_index"]],
                        "scene_frame_counter": pyboy.memory[
                            addresses["scene_frame_counter"]
                        ],
                        "start_t_cycles": pyboy._cycles(),
                        "h_vblank": pyboy.memory[addresses["h_vblank"]],
                        "h_bg_map_mode": pyboy.memory[addresses["h_bg_map_mode"]],
                        "h_cgb_pal_update": pyboy.memory[addresses["h_cgb_pal_update"]],
                        "h_oam_update": pyboy.memory[addresses["h_oam_update"]],
                        "requested_2bpp_size": pyboy.memory[addresses["requested_2bpp_size"]],
                        "requested_1bpp_size": pyboy.memory[addresses["requested_1bpp_size"]],
                        "music_playing": pyboy.memory[addresses["music_playing"]],
                        "music_id": bytes(
                            pyboy.memory[addresses["music_id"] : addresses["music_id"] + 2]
                        ).hex(),
                        "cur_sfx": pyboy.memory[addresses["cur_sfx"]],
                        "audio_state_sha256": hashlib.sha256(audio_state).hexdigest(),
                        "audio_state_hex": audio_state.hex(),
                    }

            pyboy.hook_register(
                interrupt_bank,
                interrupt_address,
                interrupt_hook,
                hook_context,
            )
        game_timer_bank, game_timer_address = symbols["GameTimer"]

        def game_timer_hook(context: dict[str, Any]) -> None:
            active = context["active_vblank_handler"]
            if active is None:
                return
            active["elapsed_t_cycles_to_game_timer"] = (
                pyboy._cycles() - active["start_t_cycles"]
            )
            active["game_timer_start_t_cycles"] = pyboy._cycles()

        def game_timer_return_hook(context: dict[str, Any]) -> None:
            active = context["active_vblank_handler"]
            if active is None or "game_timer_start_t_cycles" not in active:
                return
            active["game_timer_t_cycles"] = (
                pyboy._cycles() - active["game_timer_start_t_cycles"]
            )

        def vblank_return_hook(context: dict[str, Any]) -> None:
            active = context["active_vblank_handler"]
            if active is None or "game_timer_t_cycles" not in active:
                return
            # The hook runs immediately before `reti`; include that final
            # four-machine-cycle instruction in the complete handler total.
            active["elapsed_t_cycles_through_reti"] = (
                pyboy._cycles() - active["start_t_cycles"] + 4 * 4
            )
            active.pop("game_timer_start_t_cycles")
            vblank_handler_samples.append(active)
            context["active_vblank_handler"] = None

        pyboy.hook_register(
            game_timer_bank,
            game_timer_address,
            game_timer_hook,
            hook_context,
        )
        pyboy.hook_register(0, 0x029C, game_timer_return_hook, hook_context)
        pyboy.hook_register(0, 0x02A0, vblank_return_hook, hook_context)
        update_sound_bank, update_sound_address = symbols["_UpdateSound"]

        def update_sound_hook(context: dict[str, Any]) -> None:
            if context["active_vblank_handler"] is None:
                return
            context["active_sound_update"] = {
                "start_t_cycles": pyboy._cycles(),
                "music_parses": [],
                "helper_calls": [],
            }

        def update_sound_return_hook(context: dict[str, Any]) -> None:
            active = context["active_sound_update"]
            handler = context["active_vblank_handler"]
            if active is None or handler is None:
                return
            handler["sound_update_t_cycles"] = (
                pyboy._cycles() - active["start_t_cycles"]
            )
            handler["music_parses"] = active["music_parses"]
            handler["sound_helper_calls"] = active["helper_calls"]
            context["active_sound_update"] = None

        pyboy.hook_register(
            update_sound_bank,
            update_sound_address,
            update_sound_hook,
            hook_context,
        )
        # This instruction is immediately after VBlank_Normal's source-level
        # `call _UpdateSound`; the ROM SHA pin protects the byte address.
        pyboy.hook_register(0, 0x031D, update_sound_return_hook, hook_context)

        parse_music_bank, parse_music_address = symbols["ParseMusic"]

        def parse_channel_state() -> dict[str, Any]:
            channel = pyboy.memory[addresses["cur_channel"]]
            stride = addresses["channel2"] - addresses["channel1"]
            base = addresses["channel1"] + channel * stride
            offset = lambda name: addresses[name] - addresses["channel1"]
            tempo_address = base + offset("channel1_tempo")
            return {
                "note_length": pyboy.memory[base + offset("channel1_note_length")],
                "octave": pyboy.memory[base + offset("channel1_octave")],
                "transposition": pyboy.memory[
                    base + offset("channel1_transposition")
                ],
                "duration_modifier": pyboy.memory[
                    base + offset("channel1_duration_modifier")
                ],
                "tempo": pyboy.memory[tempo_address]
                | (pyboy.memory[tempo_address + 1] << 8),
            }

        def parse_music_hook(context: dict[str, Any]) -> None:
            if context["active_sound_update"] is None:
                return
            # ParseMusic loops back to its own entry after every command. Keep
            # the original call boundary instead of restarting the clock for
            # each command prefix.
            if context["active_music_parse"] is not None:
                active = context["active_music_parse"]
                command_start = active["active_command_start_t_cycles"]
                if command_start is not None:
                    active["command_t_cycles"].append(pyboy._cycles() - command_start)
                    active["active_command_start_t_cycles"] = None
                active["terminal_state"] = parse_channel_state()
                return
            context["active_music_parse"] = {
                "channel": pyboy.memory[addresses["cur_channel"]] + 1,
                "start_t_cycles": pyboy._cycles(),
                "commands": [],
                "command_t_cycles": [],
                "active_command_start_t_cycles": None,
                "terminal_state": parse_channel_state(),
            }

        def parse_music_command_hook(context: dict[str, Any]) -> None:
            active = context["active_music_parse"]
            if active is None:
                return
            active["commands"].append(pyboy.memory[addresses["cur_music_byte"]])
            active["active_command_start_t_cycles"] = pyboy._cycles()

        def parse_music_return_hook(context: dict[str, Any]) -> None:
            active = context["active_music_parse"]
            sound = context["active_sound_update"]
            if active is None or sound is None:
                return
            sound["music_parses"].append(
                {
                    "channel": active["channel"],
                    "elapsed_t_cycles": pyboy._cycles() - active["start_t_cycles"],
                    "commands": active["commands"],
                    "command_t_cycles": active["command_t_cycles"],
                    "terminal_music_byte": pyboy.memory[addresses["cur_music_byte"]],
                    "terminal_state": active["terminal_state"],
                }
            )
            context["active_music_parse"] = None

        pyboy.hook_register(
            parse_music_bank,
            parse_music_address,
            parse_music_hook,
            hook_context,
        )
        parse_command_bank, parse_command_address = symbols["ParseMusicCommand"]
        pyboy.hook_register(
            parse_command_bank,
            parse_command_address,
            parse_music_command_hook,
            hook_context,
        )
        # This is _UpdateSound.continue_sound_update, immediately after the
        # only source-level `call ParseMusic` in _UpdateSound.
        pyboy.hook_register(
            update_sound_bank,
            symbols["_UpdateSound.continue_sound_update"][1],
            parse_music_return_hook,
            hook_context,
        )
        sound_helper_hooks = {
            "ApplyPitchSlide": (symbols["ApplyPitchSlide"], 0x4096),
            "HandleTrackVibrato": (symbols["HandleTrackVibrato"], 0x40AD),
            "HandleNoise": (symbols["HandleNoise"], 0x40B0),
            "UpdateChannels": (symbols["UpdateChannels"], 0x40F1),
            "PlayDanger": (symbols["PlayDanger"], 0x4117),
            "FadeMusic": (symbols["FadeMusic"], 0x411A),
        }

        def sound_helper_hook(
            context: dict[str, Any], name: str
        ) -> None:
            if context["active_sound_update"] is None:
                return
            context["active_sound_helpers"][name] = {
                "channel": pyboy.memory[addresses["cur_channel"]] + 1,
                "start_t_cycles": pyboy._cycles(),
            }

        def sound_helper_return_hook(
            context: dict[str, Any], name: str
        ) -> None:
            active = context["active_sound_helpers"].pop(name, None)
            sound = context["active_sound_update"]
            if active is None or sound is None:
                return
            sound["helper_calls"].append(
                {
                    "routine": name,
                    "channel": active["channel"],
                    "elapsed_t_cycles": pyboy._cycles()
                    - active["start_t_cycles"],
                }
            )

        def set_sound_helper_hooks(context: dict[str, Any], enabled: bool) -> None:
            if context["sound_helper_hooks_registered"] == enabled:
                return
            for name, ((bank, address), return_address) in sound_helper_hooks.items():
                if enabled:
                    pyboy.hook_register(
                        bank,
                        address,
                        lambda inner, routine=name: sound_helper_hook(inner, routine),
                        context,
                    )
                    pyboy.hook_register(
                        update_sound_bank,
                        return_address,
                        lambda inner, routine=name: sound_helper_return_hook(
                            inner, routine
                        ),
                        context,
                    )
                else:
                    pyboy.hook_deregister(bank, address)
                    pyboy.hook_deregister(update_sound_bank, return_address)
            context["sound_helper_hooks_registered"] = enabled

        def intro_routine_return_hook(
            context: dict[str, Any], bank: int, address: int
        ) -> None:
            active_calls = context["active_intro_routine_calls"]
            matching_index = next(
                (
                    index
                    for index in range(len(active_calls) - 1, -1, -1)
                    if active_calls[index]["return_bank"] == bank
                    and active_calls[index]["return_address"] == address
                ),
                None,
            )
            if matching_index is None:
                return
            active = active_calls.pop(matching_index)
            end_intro_frame = current_intro_frame(context)
            intro_routine_samples.append(
                {
                    **active,
                    "end_absolute_frame": context["absolute_frame"],
                    "end_intro_frame": end_intro_frame,
                    "elapsed_t_cycles_between_hooks": pyboy._cycles()
                    - active["start_t_cycles"],
                    "frame_boundaries_crossed": end_intro_frame
                    - active["start_intro_frame"],
                    "interrupts": {
                        name: count - active["start_interrupt_counts"][name]
                        for name, count in context["interrupt_counts"].items()
                    },
                }
            )

        def intro_routine_hook(context: dict[str, Any], routine: str) -> None:
            if not context["entered"]:
                return
            stack_pointer = pyboy.register_file.SP
            return_address = (
                pyboy.memory[stack_pointer]
                | (pyboy.memory[(stack_pointer + 1) & 0xFFFF] << 8)
            )
            return_bank = (
                0
                if return_address < 0x4000
                else pyboy.memory[symbol_address(symbols, "hROMBank")]
            )
            return_location = (return_bank, return_address)
            if return_location not in context["registered_intro_routine_returns"]:
                pyboy.hook_register(
                    return_bank,
                    return_address,
                    lambda inner, bank=return_bank, address=return_address: intro_routine_return_hook(
                        inner, bank, address
                    ),
                    context,
                )
                context["registered_intro_routine_returns"].add(return_location)
            context["active_intro_routine_calls"].append(
                {
                    "routine": routine,
                    "scene": pyboy.memory[addresses["jumptable_index"]],
                    "start_absolute_frame": context["absolute_frame"],
                    "start_intro_frame": current_intro_frame(context),
                    "start_t_cycles": pyboy._cycles(),
                    "start_interrupt_counts": context["interrupt_counts"].copy(),
                    "return_bank": return_bank,
                    "return_address": return_address,
                }
            )

        for routine in (
            "Intro_ClearBGPals",
            "ClearSprites",
            "ClearTilemap",
            "Intro_DecompressRequest2bpp_64Tiles",
            "Intro_DecompressRequest2bpp_128Tiles",
            "Intro_DecompressRequest2bpp_255Tiles",
            "Request2bpp",
            "CopyBytes",
            "ClearSpriteAnims",
            "Intro_SetCGBPalUpdate",
            "Intro_ResetLYOverrides",
            "Intro_PerspectiveScrollBG",
            "Intro_LoadTilemap",
            "Intro_Scene16_AnimateSuicune",
            "Intro_ColoredSuicuneFrameSwap",
            "Intro_RustleGrass",
            "Intro_Scene20_AppearUnown",
            "Intro_Scene24_ApplyPaletteFade",
            "Intro_FadeUnownWordPals",
            "ByteFill",
            "JoyTextDelay",
            "DelayFrame",
            "DelayFrames",
            "IntroSceneJumper",
            "PlaySpriteAnimations",
            "DoNextFrameForAllSprites",
        ):
            routine_bank, routine_address = symbols[routine]
            pyboy.hook_register(
                routine_bank,
                routine_address,
                lambda context, name=routine: intro_routine_hook(context, name),
                hook_context,
            )
        # All three source-certified bridge routines place their post-call
        # `pop hl` fourteen bytes after entry. The ROM SHA pin makes this
        # byte-level hook stable while the exporter separately certifies the
        # instruction sequence from source.
        for helper in (
            "Intro_DecompressRequest2bpp_64Tiles",
            "Intro_DecompressRequest2bpp_128Tiles",
            "Intro_DecompressRequest2bpp_255Tiles",
        ):
            helper_bank, helper_address = symbols[helper]
            pyboy.hook_register(
                helper_bank,
                helper_address + 14,
                lambda context, name=helper: decompression_return_hook(context, name),
                hook_context,
            )
    try:
        intro_frame = 0
        previous_scene: int | None = None
        previous_capture: dict[str, Any] | None = None
        previous_pixels: bytes | None = None
        for absolute_frame in range(1, args.frames + 1):
            hook_context["absolute_frame"] = absolute_frame
            hook_context["frame_start_t_cycles"] = pyboy._cycles()
            pyboy.tick()
            if not hook_context["entered"]:
                continue
            intro_frame += 1
            current = capture(pyboy, absolute_frame, intro_frame, addresses)
            current_pixels = bytes(pyboy.screen.ndarray.tobytes())
            jumptable = current["jumptable_index"]
            scene = jumptable & 0x7F
            if previous_scene is None or scene != previous_scene:
                if previous_capture is not None and previous_scene is not None:
                    previous_capture["event"] = f"scene_{previous_scene + 1:02d}_last"
                    checkpoints.append(previous_capture)
                    if args.ppm_dir and previous_pixels is not None:
                        write_ppm(
                            args.ppm_dir / f"scene-{previous_scene + 1:02d}-last.ppm",
                            previous_pixels,
                        )
                current["event"] = f"scene_{scene + 1:02d}_first"
                checkpoints.append(current.copy())
                transitions.append(
                    {
                        "scene": scene + 1,
                        "absolute_frame": absolute_frame,
                        "intro_frame": intro_frame,
                    }
                )
                if args.ppm_dir:
                    write_ppm(args.ppm_dir / f"scene-{scene + 1:02d}-first.ppm", current_pixels)
                previous_scene = scene
            previous_capture = current
            previous_pixels = current_pixels
            if jumptable & JUMPTABLE_EXIT:
                previous_capture["event"] = f"scene_{scene + 1:02d}_last"
                checkpoints.append(previous_capture)
                if args.ppm_dir:
                    write_ppm(args.ppm_dir / f"scene-{scene + 1:02d}-last.ppm", current_pixels)
                break
        else:
            raise SystemExit(f"CrystalIntro did not complete within {args.frames} frames")
    finally:
        pyboy.stop()

    expected_scenes = list(range(1, 29))
    actual_scenes = [transition["scene"] for transition in transitions]
    if actual_scenes != expected_scenes:
        raise SystemExit(f"unexpected intro scene order: {actual_scenes}")
    actual_transition_frames = tuple(
        transition["intro_frame"] for transition in transitions
    )
    if actual_transition_frames != EXPECTED_SCENE_TRANSITION_FRAMES:
        raise SystemExit(
            f"unexpected intro scene transition frames: {actual_transition_frames}"
        )
    if checkpoints[-1]["intro_frame"] != EXPECTED_COMPLETION_FRAME:
        raise SystemExit(
            "unexpected CrystalIntro completion frame: "
            f"{checkpoints[-1]['intro_frame']}"
        )
    if args.timing:
        if args.all_vblanks:
            actual_vblank_frames = [
                sample["intro_frame"] for sample in vblank_handler_samples
            ]
            expected_vblank_frames = list(range(1, EXPECTED_INTRO_VBLANK_COUNT + 1))
            if actual_vblank_frames != expected_vblank_frames:
                raise SystemExit(
                    "CrystalIntro all-VBlank trace is not one complete ordered sample "
                    f"per frame: {len(actual_vblank_frames)} samples"
                )
        for sample in vblank_handler_samples:
            sound_t_cycles = sample.get("sound_update_t_cycles")
            if not isinstance(sound_t_cycles, int):
                raise SystemExit("VBlank timing sample has no completed _UpdateSound call")
            if sample["resource_symbol"] is None:
                continue
            non_audio_t_cycles = (
                sample["elapsed_t_cycles_to_game_timer"] - sound_t_cycles
            )
            if non_audio_t_cycles != EXPECTED_VBLANK_NON_AUDIO_T_CYCLES_TO_GAME_TIMER:
                raise SystemExit(
                    "unexpected VBlank_Normal non-audio timing to GameTimer: "
                    f"{non_audio_t_cycles} T-cycles"
                )
            audio_state = bytes.fromhex(sample["audio_state_hex"])
            channels_are_inactive = all(
                audio_state[channel * 50 + 3] & 1 == 0 for channel in range(8)
            )
            if (
                channels_are_inactive
                and sound_t_cycles != EXPECTED_INACTIVE_CHANNEL_SOUND_UPDATE_T_CYCLES
            ):
                raise SystemExit(
                    "unexpected all-channels-inactive _UpdateSound timing: "
                    f"{sound_t_cycles} T-cycles"
                )
            helper_calls_by_channel: dict[int, list[dict[str, Any]]] = {}
            for helper in sample["sound_helper_calls"]:
                channel = helper["channel"]
                if channel == 9:
                    continue
                helper_calls_by_channel.setdefault(channel, []).append(helper)
            composed_t_cycles = EXPECTED_INACTIVE_CHANNEL_SOUND_UPDATE_T_CYCLES
            for channel, helpers in helper_calls_by_channel.items():
                has_channel_write = any(
                    helper["routine"] == "UpdateChannels" for helper in helpers
                )
                if channel > 4:
                    composed_t_cycles += 109 * 4
                elif has_channel_write:
                    composed_t_cycles += 118 * 4
                else:
                    composed_t_cycles += 98 * 4
                composed_t_cycles += sum(
                    helper["elapsed_t_cycles"] for helper in helpers
                )
            composed_t_cycles += sum(
                parse["elapsed_t_cycles"] for parse in sample["music_parses"]
            )
            note_over_t_cycles = sound_t_cycles - composed_t_cycles
            if note_over_t_cycles < 0 or note_over_t_cycles % (12 * 4) != 0:
                raise SystemExit(
                    "stateful _UpdateSound parts do not compose to the ROM total at "
                    f"intro frame {sample['intro_frame']}: residual "
                    f"{note_over_t_cycles} T-cycles"
                )
            note_over_count = note_over_t_cycles // (12 * 4)
            if note_over_count > len(helper_calls_by_channel):
                raise SystemExit(
                    "derived _UpdateSound note-over count exceeds active channels at "
                    f"intro frame {sample['intro_frame']}"
                )
            sample["derived_note_over_count"] = note_over_count
    result = {
        "rom_sha1": digest,
        "dispatch": "unmodified CrystalIntro",
        "scene_count": len(transitions),
        "transitions": transitions,
        "checkpoints": checkpoints,
    }
    if args.timing:
        intro_start_frame_t_cycles = hook_context["intro_start_frame_t_cycles"]
        intro_entry_t_cycles = hook_context["intro_entry_t_cycles"]
        if not isinstance(intro_start_frame_t_cycles, int) or not isinstance(
            intro_entry_t_cycles, int
        ):
            raise SystemExit("CrystalIntro timing origin was not captured")
        frame_t_cycles = 70224
        for sample in [*decompressions, *intro_routine_samples]:
            start_phase = (
                sample["start_t_cycles"] - intro_start_frame_t_cycles
            ) % frame_t_cycles
            derived_boundaries = (
                start_phase + sample["elapsed_t_cycles_between_hooks"]
            ) // frame_t_cycles
            if derived_boundaries != sample["frame_boundaries_crossed"]:
                raise SystemExit(
                    "CrystalIntro frame phase does not reproduce the observed boundary count: "
                    f"{sample.get('routine', sample.get('resource_symbol'))} "
                    f"at intro frame {sample['start_intro_frame']} expected "
                    f"{sample['frame_boundaries_crossed']}, derived {derived_boundaries}"
                )
            sample["start_frame_phase_t_cycles"] = start_phase
        outer_loop_routines = {
            routine: [
                sample
                for sample in intro_routine_samples
                if sample["routine"] == routine
            ]
            for routine in (
                "JoyTextDelay",
                "IntroSceneJumper",
                "PlaySpriteAnimations",
            )
        }
        first_input = outer_loop_routines["JoyTextDelay"][0]
        entry_to_first_input_t_cycles = (
            first_input["start_t_cycles"] - intro_entry_t_cycles
        )
        if entry_to_first_input_t_cycles != (
            EXPECTED_ENTRY_TO_FIRST_INPUT_MACHINE_CYCLES + 27
        ) * 4:
            raise SystemExit(
                "CrystalIntro entry-to-first-input timing does not compose from "
                "its source body and one callback-zero LCD interrupt: "
                f"{entry_to_first_input_t_cycles} T-cycles"
            )
        joy_text_delay_body_counts: dict[int, int] = {}
        lcd_cost_options = {
            0: (0,),
            1: (27 * 4, 49 * 4),
            2: (27 * 8, (27 + 49) * 4, 49 * 8),
        }
        for sample in outer_loop_routines["JoyTextDelay"]:
            lcd_interrupts = sample["interrupts"]["lcd"]
            timer_interrupts = sample["interrupts"]["timer"]
            candidates = {
                (
                    sample["elapsed_t_cycles_between_hooks"]
                    - lcd_cost
                    - timer_interrupts * 48 * 4
                )
                // 4
                for lcd_cost in lcd_cost_options.get(lcd_interrupts, ())
                if (
                    sample["elapsed_t_cycles_between_hooks"]
                    - lcd_cost
                    - timer_interrupts * 48 * 4
                )
                in (107 * 4, 110 * 4)
            }
            if len(candidates) != 1:
                raise SystemExit(
                    "JoyTextDelay does not reduce uniquely to a source path at "
                    f"intro frame {sample['start_intro_frame']}: {candidates}"
                )
            body_machine_cycles = candidates.pop()
            sample["source_body_machine_cycles"] = body_machine_cycles
            for event in sample.get("interrupt_events", []):
                event["start_offset_t_cycles"] = (
                    event.pop("start_t_cycles") - sample["start_t_cycles"]
                )
                event["start_frame_phase_t_cycles"] = (
                    sample["start_frame_phase_t_cycles"]
                    + event["start_offset_t_cycles"]
                ) % frame_t_cycles
            joy_text_delay_body_counts[body_machine_cycles] = (
                joy_text_delay_body_counts.get(body_machine_cycles, 0) + 1
            )
        if joy_text_delay_body_counts != EXPECTED_JOY_TEXT_DELAY_BODY_COUNTS:
            raise SystemExit(
                "unexpected CrystalIntro JoyTextDelay source-path counts: "
                f"{joy_text_delay_body_counts}"
            )
        timer_interrupted_joy_calls = sum(
            sample["interrupts"]["timer"] > 0
            for sample in outer_loop_routines["JoyTextDelay"]
        )
        if timer_interrupted_joy_calls != EXPECTED_JOY_TEXT_DELAY_TIMER_CALLS:
            raise SystemExit(
                "unexpected timer-interrupted JoyTextDelay call count: "
                f"{timer_interrupted_joy_calls}"
            )

        if len(timer_interrupt_samples) != EXPECTED_TIMER_INTERRUPT_COUNT:
            raise SystemExit(
                "unexpected CrystalIntro timer interrupt count: "
                f"{len(timer_interrupt_samples)}"
            )
        ideal_timer_hook_phase = (
            timer_interrupt_samples[0]["start_t_cycles"]
            % EXPECTED_TIMER_INTERRUPT_PERIOD_T_CYCLES
        )
        if ideal_timer_hook_phase != 51_232:
            raise SystemExit(
                "unexpected CrystalIntro timer interrupt lattice phase: "
                f"{ideal_timer_hook_phase}"
            )
        observed_first_request_after_entry = (
            timer_interrupt_samples[0]["start_t_cycles"]
            - intro_entry_t_cycles
            - (5 + 4) * 4
        )
        if (
            observed_first_request_after_entry
            != EXPECTED_FIRST_TIMER_REQUEST_AFTER_INTRO_ENTRY_T_CYCLES
        ):
            raise SystemExit(
                "unexpected first CrystalIntro timer request: "
                f"{observed_first_request_after_entry} T-cycles after entry"
            )
        if any(
            sample["start_t_cycles"]
            < timer_interrupt_samples[0]["start_t_cycles"]
            + index * EXPECTED_TIMER_INTERRUPT_PERIOD_T_CYCLES
            for index, sample in enumerate(timer_interrupt_samples)
        ):
            raise SystemExit(
                "CrystalIntro timer handler entered before its hardware request lattice"
            )

        def next_hblank_offset(
            start_phase: int, after_offset: int, inclusive: bool
        ) -> int:
            absolute_phase = start_phase + after_offset
            frame = absolute_phase // frame_t_cycles
            phase = absolute_phase % frame_t_cycles
            for candidate_line in range(144):
                candidate_phase = (
                    candidate_line * 456 + EXPECTED_LCD_HBLANK_REQUEST_T_CYCLES
                )
                if candidate_phase > phase or (inclusive and candidate_phase == phase):
                    return frame * frame_t_cycles + candidate_phase - start_phase
            return (
                (frame + 1) * frame_t_cycles
                + EXPECTED_LCD_HBLANK_REQUEST_T_CYCLES
                - start_phase
            )

        instruction_reconciled_calls = 0
        for sample in outer_loop_routines["JoyTextDelay"]:
            instructions = (
                JOY_TEXT_DELAY_COMMON_INSTRUCTION_MACHINE_CYCLES
                + JOY_TEXT_DELAY_TAIL_MACHINE_CYCLES[
                    sample["source_body_machine_cycles"]
                ]
            )
            interrupt_events = sample.get("interrupt_events", [])
            call_start_phase = sample["start_frame_phase_t_cycles"]
            phase = call_start_phase
            elapsed = 0
            event_index = 0
            absolute_elapsed = sample["start_t_cycles"] - intro_entry_t_cycles
            next_lcd = next_hblank_offset(call_start_phase, 0, True)
            timer_period = EXPECTED_TIMER_INTERRUPT_PERIOD_T_CYCLES
            first_timer = EXPECTED_FIRST_TIMER_REQUEST_AFTER_INTRO_ENTRY_T_CYCLES
            if absolute_elapsed <= first_timer:
                next_timer = first_timer - absolute_elapsed
            else:
                periods = (absolute_elapsed - first_timer + timer_period - 1) // timer_period
                next_timer = first_timer + periods * timer_period - absolute_elapsed
            pending_lcd = False
            pending_timer = False

            def advance_segment(t_cycles: int) -> None:
                nonlocal elapsed, phase, next_lcd, next_timer
                nonlocal pending_lcd, pending_timer
                end = elapsed + t_cycles
                if next_lcd <= end:
                    pending_lcd = True
                    next_lcd = next_hblank_offset(call_start_phase, end, False)
                if next_timer <= end:
                    pending_timer = True
                    next_timer += timer_period
                elapsed = end
                phase = (phase + t_cycles) % frame_t_cycles

            def service_interrupts() -> None:
                nonlocal event_index, pending_lcd, pending_timer
                while pending_lcd or pending_timer:
                    interrupt = "lcd" if pending_lcd else "timer"
                    if interrupt == "lcd":
                        pending_lcd = False
                    else:
                        pending_timer = False
                    if event_index >= len(interrupt_events):
                        raise SystemExit(
                            "instruction timing predicts an extra JoyTextDelay "
                            f"{interrupt} interrupt at intro frame "
                            f"{sample['start_intro_frame']}"
                        )
                    event = interrupt_events[event_index]
                    if event["interrupt"] != interrupt:
                        raise SystemExit(
                            "JoyTextDelay interrupt priority disagrees with the ROM at "
                            f"intro frame {sample['start_intro_frame']}: expected "
                            f"{interrupt}, found {event['interrupt']}"
                        )
                    expected_hook_offset = elapsed + (5 + 4) * 4
                    if event["start_offset_t_cycles"] != expected_hook_offset:
                        raise SystemExit(
                            "JoyTextDelay interrupt admission is not at the predicted "
                            "instruction boundary on intro frame "
                            f"{sample['start_intro_frame']}: expected hook offset "
                            f"{expected_hook_offset}, found "
                            f"{event['start_offset_t_cycles']}"
                        )
                    if interrupt == "lcd":
                        handler_machine_cycles = (
                            49 if event["lcd_callback_pointer"] != 0 else 27
                        )
                    else:
                        handler_machine_cycles = 48
                    advance_segment(handler_machine_cycles * 4)
                    event_index += 1

            if next_lcd == 0:
                pending_lcd = True
                next_lcd = next_hblank_offset(call_start_phase, 0, False)
            if next_timer == 0:
                pending_timer = True
                next_timer += timer_period
            service_interrupts()
            for machine_cycles in instructions:
                advance_segment(machine_cycles * 4)
                service_interrupts()
            if event_index != len(interrupt_events) or elapsed != sample[
                "elapsed_t_cycles_between_hooks"
            ]:
                if event_index < len(interrupt_events):
                    next_event = interrupt_events[event_index]
                    detail = (
                        f"next ROM event is {next_event['interrupt']} at "
                        f"{next_event['start_offset_t_cycles']} T-cycles"
                    )
                else:
                    detail = "all ROM events were consumed"
                raise SystemExit(
                    "JoyTextDelay instruction scheduler does not reproduce the ROM call "
                    f"at intro frame {sample['start_intro_frame']}: elapsed {elapsed} "
                    f"versus {sample['elapsed_t_cycles_between_hooks']}; {detail}"
                )
            instruction_reconciled_calls += 1
        expected_instruction_reconciled_calls = EXPECTED_JOY_TEXT_DELAY_CALLS
        if instruction_reconciled_calls != expected_instruction_reconciled_calls:
            raise SystemExit(
                "unexpected JoyTextDelay instruction-reconciled call count: "
                f"expected {expected_instruction_reconciled_calls}, found "
                f"{instruction_reconciled_calls}"
            )
        # This is the return immediately following CrystalIntro's central
        # `call DelayFrame`, rather than DelayFrame calls nested in scenes.
        central_delay_return = symbols["CrystalIntro.ShutOffMusic"][1] - 3
        outer_loop_delays = [
            sample
            for sample in intro_routine_samples
            if sample["routine"] == "DelayFrame"
            and sample["return_bank"] == intro_bank
            and sample["return_address"] == central_delay_return
        ]
        if any(
            len(samples) != EXPECTED_OUTER_LOOP_ITERATIONS + 1
            for samples in outer_loop_routines.values()
        ) or len(outer_loop_delays) != EXPECTED_OUTER_LOOP_ITERATIONS:
            raise SystemExit(
                "unexpected CrystalIntro outer-loop call counts: "
                + ", ".join(
                    f"{routine}={len(samples)}"
                    for routine, samples in outer_loop_routines.items()
                )
                + f", DelayFrame={len(outer_loop_delays)}"
            )
        outer_loop_samples: list[dict[str, Any]] = []
        dispatch_ticks: dict[int, int] = {}
        for index, delay_sample in enumerate(outer_loop_delays):
            joy_sample = outer_loop_routines["JoyTextDelay"][index]
            scene_sample = outer_loop_routines["IntroSceneJumper"][index]
            sprite_sample = outer_loop_routines["PlaySpriteAnimations"][index]
            ordered = (joy_sample, scene_sample, sprite_sample, delay_sample)
            if any(
                left["start_t_cycles"]
                + left["elapsed_t_cycles_between_hooks"]
                > right["start_t_cycles"]
                for left, right in zip(ordered, ordered[1:])
            ):
                raise SystemExit(f"outer-loop call order diverged at iteration {index}")
            scene = joy_sample["scene"] & 0x7F
            dispatch_tick = dispatch_ticks.get(scene, 0) + 1
            dispatch_ticks[scene] = dispatch_tick
            end_t_cycles = (
                delay_sample["start_t_cycles"]
                + delay_sample["elapsed_t_cycles_between_hooks"]
            )
            elapsed_t_cycles = end_t_cycles - joy_sample["start_t_cycles"]
            frame_boundaries_crossed = (
                joy_sample["start_frame_phase_t_cycles"] + elapsed_t_cycles
            ) // frame_t_cycles
            gaps = []
            for left, right in zip(ordered, ordered[1:]):
                left_end_interrupt_counts = {
                    name: left["start_interrupt_counts"][name]
                    + left["interrupts"][name]
                    for name in left["start_interrupt_counts"]
                }
                gaps.append(
                    {
                        "after": left["routine"],
                        "before": right["routine"],
                        "elapsed_t_cycles": right["start_t_cycles"]
                        - (
                            left["start_t_cycles"]
                            + left["elapsed_t_cycles_between_hooks"]
                        ),
                        "interrupts": {
                            name: right["start_interrupt_counts"][name] - count
                            for name, count in left_end_interrupt_counts.items()
                        },
                    }
                )
            outer_loop_samples.append(
                {
                    "iteration": index + 1,
                    "scene": scene,
                    "dispatch_tick": dispatch_tick,
                    "start_intro_frame": joy_sample["start_intro_frame"],
                    "end_intro_frame": delay_sample["end_intro_frame"],
                    "start_frame_phase_t_cycles": joy_sample[
                        "start_frame_phase_t_cycles"
                    ],
                    "elapsed_t_cycles": elapsed_t_cycles,
                    "frame_boundaries_crossed": frame_boundaries_crossed,
                    "calls": [
                        {
                            "routine": sample["routine"],
                            "start_offset_t_cycles": sample["start_t_cycles"]
                            - joy_sample["start_t_cycles"],
                            "elapsed_t_cycles": sample[
                                "elapsed_t_cycles_between_hooks"
                            ],
                            "frame_boundaries_crossed": sample[
                                "frame_boundaries_crossed"
                            ],
                        }
                        for sample in ordered
                    ],
                    "gaps": gaps,
                }
            )
        crossed_outer_loop_boundaries = sum(
            sample["frame_boundaries_crossed"] for sample in outer_loop_samples
        )
        if crossed_outer_loop_boundaries != EXPECTED_OUTER_LOOP_FRAME_BOUNDARIES:
            raise SystemExit(
                "unexpected CrystalIntro outer-loop frame boundary total: "
                f"{crossed_outer_loop_boundaries}"
            )
        expected_gap_body_t_cycles = (84, 192, 196)
        for sample in outer_loop_samples:
            for gap_index, gap in enumerate(sample["gaps"]):
                interrupts = gap["interrupts"]
                if any(
                    count
                    for name, count in interrupts.items()
                    if name not in ("lcd", "timer")
                ) or interrupts["lcd"] not in (0, 1) or interrupts["timer"] not in (0, 1):
                    raise SystemExit(
                        "unexpected interrupt topology in CrystalIntro outer-loop gap "
                        f"{sample['iteration']}:{gap_index}: {interrupts}"
                    )
                interrupt_cost_options = (0,)
                if interrupts["lcd"] == 1:
                    interrupt_cost_options = (27 * 4, 49 * 4)
                if interrupts["timer"] == 1:
                    interrupt_cost_options = tuple(
                        cost + 48 * 4 for cost in interrupt_cost_options
                    )
                if not any(
                    gap["elapsed_t_cycles"] - interrupt_cost
                    == expected_gap_body_t_cycles[gap_index]
                    for interrupt_cost in interrupt_cost_options
                ):
                    raise SystemExit(
                        "CrystalIntro outer-loop gap does not reduce to its source body: "
                        f"iteration {sample['iteration']} gap {gap_index}"
                    )
        result["timing_origin"] = {
            "frame_t_cycles": frame_t_cycles,
            "intro_start_frame_t_cycles": intro_start_frame_t_cycles,
            "intro_entry_t_cycles": intro_entry_t_cycles,
            "intro_entry_phase_t_cycles": intro_entry_t_cycles
            - intro_start_frame_t_cycles,
            "timer_interrupt_period_t_cycles": EXPECTED_TIMER_INTERRUPT_PERIOD_T_CYCLES,
            "first_timer_request_after_intro_entry_t_cycles": (
                EXPECTED_FIRST_TIMER_REQUEST_AFTER_INTRO_ENTRY_T_CYCLES
            ),
        }
        result["decompressions"] = decompressions
        result["intro_routine_samples"] = intro_routine_samples
        result["outer_loop_samples"] = outer_loop_samples
        result["timer_interrupt_samples"] = timer_interrupt_samples
        result["vblank_handler_samples"] = vblank_handler_samples
    print(
        json.dumps(
            result,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
