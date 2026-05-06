
import { DAY_NAME_LABELS, loadInitializeEventsConfig } from "../common";
import { bug_contest_judging } from "@pokecrystal/core/engine/world/special-events/bug-contest";
import { pokemon_center_pc } from "@pokecrystal/core/engine/world/special-events/pc";
import { Event, showText, waitForInput } from "@pokecrystal/core/engine/events/events";
import { SetMapSceneCommand } from "../commands/events";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import {
    bugContestCopyContestants,
    closeTextBox,
    currentLandmarkName,
    determineBugContestRank,
    giveVerboseItem,
    openTextBox,
    attemptCoinPurchase,
    playerHasCoinCase,
    queueSpecialPhoneCall,
    recordWarpDestination,
    runPhoneTextScript,
    setEngineFlag,
    setEventFlag,
    showLabelledText,
} from "./helpers";
import type { ScriptRunner } from "../runner";
import type { PokemonCenterSystem } from "@pokecrystal/core/ui/screens/pokemon-center";

type AudioEngineLike = {
  play_sound?: (name: string) => void;
  playSound?: (name: string) => void;
  play_music?: (name: string, role?: string | { role?: string }) => void;
  playMusic?: (name: string, role: string) => void;
  restart_map_music?: () => void;
  restartMapMusic?: () => void;
};

type StrengthOverworld = {
  _handle_hm?: (moveName: string, x: number, y: number, playerState: unknown) => boolean | Promise<boolean>;
  handle_strength?: (x: number, y: number) => boolean;
  player_state?: unknown;
};

const finishStrengthBoulderScript = (
    runner: ScriptRunner,
    prompt: string,
    used: boolean,
): void => {
    if (used) {
        const species = String((runner.overworld as { game_state?: { wram?: { wStrengthSpecies?: unknown } } } | null)?.game_state?.wram?.wStrengthSpecies ?? "").trim();
        const actorName = species || "POKEMON";
        if (!runner.string_buffers) {
            runner.string_buffers = {};
        }
        runner.string_buffers.STRING_BUFFER_1 = actorName;
        runner.string_buffers.STRING_BUFFER_2 = actorName;
        runner.string_buffers.STRING_BUFFER_3 = actorName;
        runner.string_buffers.STRING_BUFFER_4 = actorName;
    }
    const useText = used ? showLabelledText(runner, "UseStrengthText", { wait: false }) : "";
    closeTextBox(runner);
    runner.last_value = { strength_boulder: { prompt, used, message: useText } };
    runner.resume?.();
};

function run_strength_boulder_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::StrengthBoulderScript
    const eventManager = runner.event_manager;
    const overworld = runner.overworld as StrengthOverworld | null;
    openTextBox(runner);
    const askText = showLabelledText(runner, "AskStrengthText", { wait: false });
    runner.last_value = { strength_boulder: { prompt: askText, used: false } };
    runner.pause?.();
    waitForInput(eventManager);
    eventManager.dispatch(
        new Event("prompt_yes_no", {
            callback: (accepted: boolean) => {
                const confirmed = Boolean(accepted);
                runner.last_yes_no_result = confirmed;
                runner.last_condition_result = confirmed;
                if (!confirmed) {
                    closeTextBox(runner);
                    runner.last_value = { strength_boulder: { prompt: askText, used: false } };
                    runner.resume?.();
                    return;
                }
                const handleHm = overworld?._handle_hm;
                let result: boolean | Promise<boolean> = false;
                if (typeof handleHm === "function") {
                    result = handleHm.call(overworld, "Strength", 0, 0, overworld?.player_state);
                } else if (typeof overworld?.handle_strength === "function") {
                    result = overworld.handle_strength(0, 0);
                }
                Promise.resolve(result).then(
                    (used) => finishStrengthBoulderScript(runner, askText, Boolean(used)),
                    () => finishStrengthBoulderScript(runner, askText, false)
                );
            },
        })
    );
}

function run_bug_contest_results_warp_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::BugContestResultsWarpScript
    bugContestCopyContestants(runner);
    setEventFlag(runner, "EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_CONTEST_DAY", true);
    setEventFlag(runner, "EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_NOT_CONTEST_DAY", false);
    setEventFlag(runner, "EVENT_WARPED_FROM_ROUTE_35_NATIONAL_PARK_GATE", true);
    recordWarpDestination(runner, "ROUTE_36_NATIONAL_PARK_GATE", 0, 4);
    const details: Record<string, unknown> =
        runner?.last_value && typeof runner.last_value === "object" ? (runner.last_value as Record<string, unknown>) : {};
    details["bug_contest_warp"] = {
        movement: "Movement_ContestResults_WalkAfterWarp",
        target: "ROUTE_36_NATIONAL_PARK_GATE",
    };
    runner.last_value = details;
}

function run_bug_contest_results_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::BugContestResultsScript
    setEngineFlag(runner, "ENGINE_BUG_CONTEST_TIMER", false);
    for (const flag of [
        "EVENT_WARPED_FROM_ROUTE_35_NATIONAL_PARK_GATE",
        "EVENT_CONTEST_OFFICER_HAS_SUN_STONE",
        "EVENT_CONTEST_OFFICER_HAS_EVERSTONE",
        "EVENT_CONTEST_OFFICER_HAS_GOLD_BERRY",
        "EVENT_CONTEST_OFFICER_HAS_BERRY",
    ]) {
        setEventFlag(runner, flag, false);
    }

    openTextBox(runner);
    const introText = showLabelledText(runner, "ContestResults_ReadyToJudgeText");
    if (!runner.variables || !("_bug_contest_rank" in runner.variables)) {
        bug_contest_judging(runner.game_state, {
            runner,
            overworld: runner.overworld,
            event_manager: runner.event_manager,
        });
    }
    const rank = determineBugContestRank(runner);

    let rewardItem = "BERRY";
    if (rank === 1) {
        rewardItem = "SUN_STONE";
        setEventFlag(runner, "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1", true);
        showLabelledText(runner, "ContestResults_PlayerWonAPrizeText");
        giveVerboseItem(runner, rewardItem);
    } else if (rank === 2) {
        rewardItem = "EVERSTONE";
        showLabelledText(runner, "ContestResults_PlayerWonAPrizeText");
        giveVerboseItem(runner, rewardItem);
    } else if (rank === 3) {
        rewardItem = "GOLD_BERRY";
        showLabelledText(runner, "ContestResults_PlayerWonAPrizeText");
        giveVerboseItem(runner, rewardItem);
    } else {
        showLabelledText(runner, "ContestResults_ConsolationPrizeText");
        giveVerboseItem(runner, rewardItem);
        showLabelledText(runner, "ContestResults_DidNotWinText");
    }

    showLabelledText(runner, "ContestResults_JoinUsNextTimeText");
    if (runner?.game_state?.wram?.event_flags?.EVENT_LEFT_MONS_WITH_CONTEST_OFFICER) {
        showLabelledText(runner, "ContestResults_ReturnPartyText");
        setEventFlag(runner, "EVENT_LEFT_MONS_WITH_CONTEST_OFFICER", false);
    }
    closeTextBox(runner);

    for (let index = 1; index <= 10; index += 1) {
        setEventFlag(runner, `EVENT_BUG_CATCHING_CONTESTANT_${index}A`, true);
        setEventFlag(runner, `EVENT_BUG_CATCHING_CONTESTANT_${index}B`, true);
    }

    setEngineFlag(runner, "ENGINE_DAILY_BUG_CONTEST", true);
    runner.last_value = {
        bug_contest: {
            intro: introText,
            rank,
            reward: rewardItem,
        },
    };
    runner.last_condition_result = true;
}

function run_initialize_events_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::InitializeEventsScript
    const config = loadInitializeEventsConfig();
    for (const flag of config.eventFlags) {
        setEventFlag(runner, flag, true);
    }
    for (const flag of config.engineFlags) {
        if (runner?.game_state?.wram?.engine_flags) {
            runner.game_state.wram.engine_flags[flag] = true;
        }
    }
    for (const [spriteId, replacement] of Object.entries(config.variableSprites)) {
        if (runner?.game_state?.wram?.variable_sprites) {
            runner.game_state.wram.variable_sprites[spriteId] = replacement;
        }
    }
}

function run_ask_number_1f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::AskNumber1FScript
    runPhoneTextScript(runner, "AskNumber1Text", { female: true });
}

function run_ask_number_1m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::AskNumber1MScript
    runPhoneTextScript(runner, "AskNumber1Text", { female: false });
}

function run_ask_number_2f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::AskNumber2FScript
    runPhoneTextScript(runner, "AskNumber2Text", { female: true });
}

function run_ask_number_2m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::AskNumber2MScript
    runPhoneTextScript(runner, "AskNumber2Text", { female: false });
}

function run_day_to_text_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::DayToTextScript
    let weekday = runner?.variables?.VAR_WEEKDAY;
    if (weekday === null || weekday === undefined) {
        weekday = runner?.game_state?.sram?.day_of_week;
    }
    let index = 0;
    if (weekday !== null && weekday !== undefined) {
        const parsed = Number(weekday);
        if (Number.isFinite(parsed)) {
            const total = DAY_NAME_LABELS.length;
            index = ((parsed % total) + total) % total;
        }
    }
    const dayName = DAY_NAME_LABELS[index] ?? DAY_NAME_LABELS[0];
    if (!runner.string_buffers) {
        runner.string_buffers = {};
    }
    runner.string_buffers["STRING_BUFFER_3"] = dayName;
    runner.last_value = { day_to_text: dayName };
}

function run_difficult_bookshelf_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::DifficultBookshelfScript
    openTextBox(runner);
    const message = showLabelledText(runner, "DifficultBookshelfText");
    closeTextBox(runner);
    runner.last_value = { bookshelf: message };
}

function run_elevator_button_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::ElevatorButtonScript
    const audioEngine: AudioEngineLike | null = runner?.overworld?.audio_engine ?? runner?.overworld?.audioEngine ?? null;
    const sounds = ["SFX_READ_TEXT_2", "SFX_ELEVATOR_END"];
    if (audioEngine?.play_sound) {
        audioEngine.play_sound(sounds[0]);
    } else if (audioEngine?.playSound) {
        audioEngine.playSound(sounds[0]);
    }
    runner.last_sound_effect = sounds[0];
    if (audioEngine?.play_sound) {
        audioEngine.play_sound(sounds[1]);
    } else if (audioEngine?.playSound) {
        audioEngine.playSound(sounds[1]);
    }
    runner.last_sound_effect = sounds[1];
    runner.last_value = {
        elevator_button: {
            sounds,
            delay_frames: 15,
        },
    };
}

function run_gift_f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::GiftFScript
    runPhoneTextScript(runner, "GiftText", { female: true });
}

function run_gift_m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::GiftMScript
    runPhoneTextScript(runner, "GiftText", { female: false });
}

function run_goldenrod_rockets_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::GoldenrodRocketsScript
    setEventFlag(runner, "EVENT_GOLDENROD_CITY_ROCKET_TAKEOVER", false);
    runner.last_value = { goldenrod_rockets: "cleared" };
}

function run_pokecenter_nurse_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::PokecenterNurseScript
    const system = runner.pokemon_center ?? null;
    if (!system) {
        throw new Error("PokecenterNurseScript requires a PokemonCenterSystem.");
    }
    const eventManager = runner.event_manager ?? null;
    if (!eventManager) {
        throw new Error("PokecenterNurseScript requires an EventManager.");
    }
    const overworld = runner.overworld ?? null;
    system.runNurseInteraction(
        runner,
        eventManager,
        overworld as Parameters<PokemonCenterSystem["runNurseInteraction"]>[2],
    );
}

function run_pc_script(runner: ScriptRunner): void {
    // ASM: engine/events/std_scripts.asm::PCScript dispatches to the Pokemon Center PC special.
    openTextBox(runner);
    const result = pokemon_center_pc(runner.game_state, {
        runner,
        overworld: runner.overworld,
        event_manager: runner.event_manager,
    });
    runner.pause?.();
    result
        .then((summary) => {
            runner.last_value = summary;
            runner.last_condition_result = String(summary.selection_name ?? "").toUpperCase() !== "TURN OFF";
        })
        .finally(() => {
            runner.resume?.();
        });
}

function run_magazine_bookshelf_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::MagazineBookshelfScript
    openTextBox(runner);
    const message = showLabelledText(runner, "MagazineBookshelfText");
    closeTextBox(runner);
    runner.last_value = { bookshelf: message };
}

function run_gym_statue_1_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::GymStatue1Script
    const location = currentLandmarkName(runner);
    runner.string_buffers["STRING_BUFFER_3"] = location;
    openTextBox(runner);
    const message = showLabelledText(runner, "GymStatue_CityGymText");
    closeTextBox(runner);
    runner.last_value = {
        gym_statue: { variant: 1, location, message },
    };
}

function run_gym_statue_2_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::GymStatue2Script
    const location = currentLandmarkName(runner);
    runner.string_buffers["STRING_BUFFER_3"] = location;
    openTextBox(runner);
    const bufferSnapshot = { ...runner.string_buffers };
    const leaderValue = bufferSnapshot.STRING_BUFFER_4 ?? "";
    let cityMessage = "";
    let winnersMessage = "";
    try {
        runner.string_buffers = { STRING_BUFFER_3: location };
        cityMessage = showLabelledText(runner, "GymStatue_CityGymText");
        runner.string_buffers = { STRING_BUFFER_4: leaderValue };
        winnersMessage = showLabelledText(runner, "GymStatue_WinningTrainersText");
    } finally {
        runner.string_buffers = bufferSnapshot;
    }
    closeTextBox(runner);
    runner.last_value = {
        gym_statue: {
            variant: 2,
            location,
            messages: [cityMessage, winnersMessage],
        },
    };
}

function run_number_accepted_f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::NumberAcceptedFScript
    runPhoneTextScript(runner, "NumberAcceptedText", { female: true });
}

function run_number_accepted_m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::NumberAcceptedMScript
    runPhoneTextScript(runner, "NumberAcceptedText", { female: false });
}

function run_number_declined_f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::NumberDeclinedFScript
    runPhoneTextScript(runner, "NumberDeclinedText", { female: true });
}

function run_number_declined_m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::NumberDeclinedMScript
    runPhoneTextScript(runner, "NumberDeclinedText", { female: false });
}

function run_pack_full_f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::PackFullFScript
    runPhoneTextScript(runner, "PackFullText", { female: true });
}

function run_pack_full_m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::PackFullMScript
    runPhoneTextScript(runner, "PackFullText", { female: false });
}

function run_phone_full_f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::PhoneFullFScript
    runPhoneTextScript(runner, "PhoneFullText", { female: true });
}

function run_phone_full_m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::PhoneFullMScript
    runPhoneTextScript(runner, "PhoneFullText", { female: false });
}

function run_picture_bookshelf_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::PictureBookshelfScript
    openTextBox(runner);
    const message = showLabelledText(runner, "PictureBookshelfText");
    closeTextBox(runner);
    runner.last_value = { bookshelf: message };
}

function run_radio_tower_rockets_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::RadioTowerRocketsScript
    setEngineFlag(runner, "ENGINE_ROCKETS_IN_RADIO_TOWER", true);
    setEventFlag(runner, "EVENT_GOLDENROD_CITY_CIVILIANS", true);
    setEventFlag(runner, "EVENT_RADIO_TOWER_BLACKBELT_BLOCKS_STAIRS", true);
    setEventFlag(runner, "EVENT_RADIO_TOWER_ROCKET_TAKEOVER", false);
    setEventFlag(runner, "EVENT_USED_THE_CARD_KEY_IN_THE_RADIO_TOWER", false);
    setEventFlag(runner, "EVENT_MAHOGANY_TOWN_POKEFAN_M_BLOCKS_EAST", true);
    queueSpecialPhoneCall(runner, "SPECIALCALL_WEIRDBROADCAST");
    const setScene = new SetMapSceneCommand("MAHOGANY_TOWN", "SCENE_MAHOGANYTOWN_NOOP");
    setScene.runner = runner;
    setScene.execute(runner.game_state, runner.event_manager, runner.overworld);
    runner.last_value = {
        radio_tower: {
            call: "SPECIALCALL_WEIRDBROADCAST",
            map_scene: "SCENE_MAHOGANYTOWN_NOOP",
        },
    };
}

function run_town_map_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::TownMapScript
    openTextBox(runner);
    const message = showLabelledText(runner, "LookTownMapText");
    runner.event_manager?.dispatch(new Event("show_town_map", { source: "TownMapScript", runner }));
    runner.last_value = { town_map: { opened: true, message } };
    closeTextBox(runner);
}

function run_tv_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::TVScript
    openTextBox(runner);
    const message = showLabelledText(runner, "TVText");
    closeTextBox(runner);
    runner.last_value = { tv: { message } };
}

function run_registered_number_f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::RegisteredNumberFScript
    openTextBox(runner);
    const message = showLabelledText(runner, "RegisteredNumber2Text");
    const audioEngine: AudioEngineLike | null = runner?.overworld?.audio_engine ?? runner?.overworld?.audioEngine ?? null;
    if (audioEngine?.play_sound) {
        audioEngine.play_sound("SFX_REGISTER_PHONE_NUMBER");
    } else if (audioEngine?.playSound) {
        audioEngine.playSound("SFX_REGISTER_PHONE_NUMBER");
    }
    runner.last_sound_effect = "SFX_REGISTER_PHONE_NUMBER";
    closeTextBox(runner);
    runner.last_value = { phone_registration: { message, female: true } };
}

function run_registered_number_m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::RegisteredNumberMScript
    openTextBox(runner);
    const message = showLabelledText(runner, "RegisteredNumber1Text");
    const audioEngine: AudioEngineLike | null = runner?.overworld?.audio_engine ?? runner?.overworld?.audioEngine ?? null;
    if (audioEngine?.play_sound) {
        audioEngine.play_sound("SFX_REGISTER_PHONE_NUMBER");
    } else if (audioEngine?.playSound) {
        audioEngine.playSound("SFX_REGISTER_PHONE_NUMBER");
    }
    runner.last_sound_effect = "SFX_REGISTER_PHONE_NUMBER";
    closeTextBox(runner);
    runner.last_value = { phone_registration: { message, female: false } };
}

function run_rematch_f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::RematchFScript
    runPhoneTextScript(runner, "RematchText", { female: true });
}

function run_rematch_gift_f_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::RematchGiftFScript
    runPhoneTextScript(runner, "RematchGiftText", { female: true });
}

function run_rematch_gift_m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::RematchGiftMScript
    runPhoneTextScript(runner, "RematchGiftText", { female: false });
}

function run_rematch_m_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::RematchMScript
    runPhoneTextScript(runner, "RematchText", { female: false });
}

function run_receive_item_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::ReceiveItemScript
    const rawItemName = runner.string_buffers?.STRING_BUFFER_4 ?? "";
    const itemName = rawItemName.replace(/\s+/g, " ").trim();
    if (!itemName) {
        throw new Error("ReceiveItemScript requires STRING_BUFFER_4 to contain the received item name.");
    }
    const playerName = runner.game_state?.sram?.player_name || "PLAYER";
    const message = `${playerName} received\n${itemName}.`;
    const audioEngine: AudioEngineLike | null = runner?.overworld?.audio_engine ?? runner?.overworld?.audioEngine ?? null;
    if (audioEngine?.play_sound) {
        audioEngine.play_sound("SFX_ITEM");
    } else if (audioEngine?.playSound) {
        audioEngine.playSound("SFX_ITEM");
    }
    runner.last_sound_effect = "SFX_ITEM";
    if (runner.event_manager) {
        showText(runner.event_manager, message, { auto_close_after_wait: true });
        runner.pause?.();
        waitForInput(runner.event_manager);
    }
}

function run_find_item_in_ball_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/misc_scripts.asm::FindItemInBallScript
    const eventManager = runner.event_manager;
    if (!eventManager) {
        return;
    }
    openTextBox(runner);
    showLabelledText(runner, "FoundItemText", { wait: false });
    const audioEngine: AudioEngineLike | null = runner?.overworld?.audio_engine ?? runner?.overworld?.audioEngine ?? null;
    if (audioEngine?.play_sound) {
        audioEngine.play_sound("SFX_ITEM");
    } else if (audioEngine?.playSound) {
        audioEngine.playSound("SFX_ITEM");
    }
    runner.last_sound_effect = "SFX_ITEM";
    if (runner.last_condition_result) {
        const pauseMs = 60 * GB_FRAME_DURATION_MS;
        runner.pause?.();
        setTimeout(() => {
            closeTextBox(runner);
            runner.resume?.();
        }, pauseMs);
        return;
    }
    runner.pause?.();
    waitForInput(eventManager);
    showLabelledText(runner, "CantCarryItemText", { wait: false });
    runner.pause?.();
    waitForInput(eventManager);
    closeTextBox(runner);
}

function run_game_corner_coin_vendor_script(runner: ScriptRunner): void {
    // ASM: pokecrystal_disassembly/engine/events/std_scripts.asm::GameCornerCoinVendorScript
    const eventManager = runner.event_manager;
    openTextBox(runner);
    if (!playerHasCoinCase(runner)) {
        showLabelledText(runner, "CoinVendor_NoCoinCaseText");
        closeTextBox(runner);
        runner.last_value = { coin_vendor: { status: "no_coin_case" } };
        return;
    }

    showLabelledText(runner, "CoinVendor_WelcomeText", { wait: false });
    showLabelledText(runner, "CoinVendor_IntroText", { wait: false });
    runner.last_value = { coin_vendor: { status: "prompt" } };
    if (!eventManager) {
        return;
    }
    runner.pause?.();
    waitForInput(eventManager);
    eventManager.dispatch(
        new Event("prompt_yes_no", {
            callback: (accepted: boolean) => {
                if (!accepted) {
                    showLabelledText(runner, "CoinVendor_CancelText");
                    closeTextBox(runner);
                    runner.last_value = { coin_vendor: { status: "cancelled" } };
                    runner.resume?.();
                    return;
                }
                const result = attemptCoinPurchase(runner, 50, 1000);
                closeTextBox(runner);
                runner.last_value = { coin_vendor: result };
                runner.resume?.();
            },
        })
    );
}

export const STANDARD_SCRIPT_HANDLERS: Record<string, (runner: ScriptRunner) => void> = {
    AskNumber1FScript: run_ask_number_1f_script,
    AskNumber1MScript: run_ask_number_1m_script,
    AskNumber2FScript: run_ask_number_2f_script,
    AskNumber2MScript: run_ask_number_2m_script,
    BugContestResultsWarpScript: run_bug_contest_results_warp_script,
    BugContestResultsScript: run_bug_contest_results_script,
    DayToTextScript: run_day_to_text_script,
    DifficultBookshelfScript: run_difficult_bookshelf_script,
    ElevatorButtonScript: run_elevator_button_script,
    GiftFScript: run_gift_f_script,
    GiftMScript: run_gift_m_script,
    FindItemInBallScript: run_find_item_in_ball_script,
    GameCornerCoinVendorScript: run_game_corner_coin_vendor_script,
    GoldenrodRocketsScript: run_goldenrod_rockets_script,
    GymStatue1Script: run_gym_statue_1_script,
    GymStatue2Script: run_gym_statue_2_script,
    InitializeEventsScript: run_initialize_events_script,
    NumberAcceptedFScript: run_number_accepted_f_script,
    NumberAcceptedMScript: run_number_accepted_m_script,
    NumberDeclinedFScript: run_number_declined_f_script,
    NumberDeclinedMScript: run_number_declined_m_script,
    PackFullFScript: run_pack_full_f_script,
    PackFullMScript: run_pack_full_m_script,
    PCScript: run_pc_script,
    PokecenterNurseScript: run_pokecenter_nurse_script,
    PictureBookshelfScript: run_picture_bookshelf_script,
    PhoneFullFScript: run_phone_full_f_script,
    PhoneFullMScript: run_phone_full_m_script,
    MagazineBookshelfScript: run_magazine_bookshelf_script,
    RadioTowerRocketsScript: run_radio_tower_rockets_script,
    RegisteredNumberFScript: run_registered_number_f_script,
    RegisteredNumberMScript: run_registered_number_m_script,
    RematchFScript: run_rematch_f_script,
    RematchGiftFScript: run_rematch_gift_f_script,
    RematchGiftMScript: run_rematch_gift_m_script,
    RematchMScript: run_rematch_m_script,
    ReceiveItemScript: run_receive_item_script,
    StrengthBoulderScript: run_strength_boulder_script,
    TownMapScript: run_town_map_script,
    TVScript: run_tv_script,
};
