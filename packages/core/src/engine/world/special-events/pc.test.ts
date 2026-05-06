
import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import { createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { players_house_pc, pokemon_center_pc, BillPC, hall_of_fame_terminal } from "./pc";
import { SPECIAL_FUNCTIONS } from "./registry";
import { PokemonCenterPCSession } from "@pokecrystal/core/ui/menus/pc-menu";
import * as pcHelpers from './pc-helpers';

jest.mock('@pokecrystal/core/ui/menus/pc-menu', () => ({
    PokemonCenterPCSession: jest.fn().mockImplementation(() => ({
        runAsync: jest.fn().mockResolvedValue({
            selection_index: 0,
            selection_name: 'TURN OFF',
        }),
        runHallOfFame: jest.fn().mockResolvedValue({ success: true }),
        runHallOfFameInteractive: jest.fn().mockResolvedValue({ success: true, interactive: true }),
        runHallOfFameInteractiveAsync: jest.fn().mockResolvedValue({ success: true, interactive: true }),
    })),
}));

jest.mock('./pc-helpers', () => ({
    pcHubEntries: jest.fn().mockReturnValue([{ label: "BILL's PC" }, { label: "PLAYER's PC" }, { label: "TURN OFF" }]),
}));

describe("players_house_pc", () => {
    it("clears decorations and updates runner state when decorations changed", () => {
        const gameState = createInitialGameState();
        gameState.wram.maptile_decorations_visible = true;
        const runner = createScriptRunnerStub({
            variables: { _players_house_pc_changed_decorations: true },
            _script_stack: [{ name: "players_house_pc", commands: [], index: 0, allowFallthrough: false }],
            last_condition_result: false,
            last_value: null,
        }) as unknown as ScriptRunner & { _script_stack: Array<{ allowFallthrough: boolean }> };

        const result = players_house_pc(gameState, { runner });

        expect(result).toBe(true);
        expect(gameState.wram.maptile_decorations_visible).toBe(false);
        expect(runner.last_condition_result).toBe(true);
        expect(runner._script_stack[0].allowFallthrough).toBe(true);
        expect(runner.last_value).toEqual({
            pc: expect.objectContaining({
                changed_decorations: true,
            }),
        });
    });

    it("records the ASM boot text instead of leaking the text label", () => {
        const gameState = createInitialGameState();
        const runner = createScriptRunnerStub({
            variables: {},
            _script_stack: [{ name: "players_house_pc", commands: [], index: 0, allowFallthrough: false }],
            last_condition_result: false,
            last_value: null,
        }) as unknown as ScriptRunner & { last_value: { pc: { boot_text: string } } };

        const result = players_house_pc(gameState, { runner });

        expect(result).toBe(false);
        expect(runner.last_value.pc.boot_text).toBe("<PLAYER> turned on\nthe PC.");
    });
});

describe("special registry", () => {
    it("registers PokemonCenterPC", () => {
        expect(SPECIAL_FUNCTIONS.PokemonCenterPC).toBe(pokemon_center_pc);
    });
});

describe('pokemon_center_pc', () => {
    it('should run the Pokemon Center PC session and update runner state', async () => {
        const gameState = createInitialGameState();
        const runner = createScriptRunnerStub({
            variables: {},
            last_condition_result: true,
            last_value: null,
        }) as unknown as ScriptRunner;
        const overworld = {
            ui: {},
            dialogue: {},
            audio_engine: {},
            draw: jest.fn(),
            update: jest.fn(),
            script_runner: runner,
        } as any;

        const result = await pokemon_center_pc(gameState, { runner, overworld });

        expect(PokemonCenterPCSession).toHaveBeenCalled();
        expect(result.selection_name).toBe('TURN OFF');
        expect(runner.last_condition_result).toBe(false);
        expect(runner.last_value).toHaveProperty('pc');
    });

    it('captures and restores overworld input during the session', async () => {
        const gameState = createInitialGameState();
        const runner = createScriptRunnerStub({
            variables: {},
            last_condition_result: true,
            last_value: null,
        }) as unknown as ScriptRunner;
        const overworld = {
            ui: {},
            dialogue: {},
            audio_engine: {},
            draw: jest.fn(),
            update: jest.fn(),
            script_runner: runner,
            input_capture_active: false,
        } as any;

        let resolveSession: ((value: Record<string, unknown>) => void) | null = null;
        const pending = new Promise<Record<string, unknown>>((resolve) => {
            resolveSession = resolve;
        });
        (PokemonCenterPCSession as jest.Mock).mockImplementationOnce(() => ({
            runAsync: jest.fn().mockReturnValue(pending),
            runHallOfFame: jest.fn().mockResolvedValue({ success: true }),
            runHallOfFameInteractive: jest.fn().mockResolvedValue({ success: true, interactive: true }),
            runHallOfFameInteractiveAsync: jest.fn().mockResolvedValue({ success: true, interactive: true }),
        }));

        const promise = pokemon_center_pc(gameState, { runner, overworld });

        expect(overworld.input_capture_active).toBe(true);

        resolveSession?.({ selection_index: 0, selection_name: 'TURN OFF' });
        await promise;

        expect(overworld.input_capture_active).toBe(false);
    });

    it("uses the ASM PC result text instead of fabricated English", async () => {
        const gameState = createInitialGameState();
        const runner = createScriptRunnerStub({
            variables: {},
            last_condition_result: true,
            last_value: null,
        }) as unknown as ScriptRunner & { last_value: { pc: { result_text: string } } };
        const overworld = {
            ui: {},
            dialogue: {},
            audio_engine: {},
            draw: jest.fn(),
            update: jest.fn(),
            script_runner: runner,
        } as any;

        (PokemonCenterPCSession as jest.Mock).mockImplementationOnce(() => ({
            runAsync: jest.fn().mockResolvedValue({
                selection_index: 0,
                selection_name: "PLAYER'S PC",
            }),
            runHallOfFame: jest.fn().mockResolvedValue({ success: true }),
            runHallOfFameInteractive: jest.fn().mockResolvedValue({ success: true, interactive: true }),
            runHallOfFameInteractiveAsync: jest.fn().mockResolvedValue({ success: true, interactive: true }),
        }));

        const result = await pokemon_center_pc(gameState, { runner, overworld });

        expect(result.result_text).toBe("Accessed own PC.\n\nItem Storage\nSystem opened.");
        expect(runner.last_value.pc.result_text).toBe("Accessed own PC.\n\nItem Storage\nSystem opened.");
    });

    it("throws when the session returns a non-ASM PC selection label", async () => {
        const gameState = createInitialGameState();
        const runner = createScriptRunnerStub({
            variables: {},
            last_condition_result: true,
            last_value: null,
        }) as unknown as ScriptRunner;
        const overworld = {
            ui: {},
            dialogue: {},
            audio_engine: {},
            draw: jest.fn(),
            update: jest.fn(),
            script_runner: runner,
        } as any;

        (PokemonCenterPCSession as jest.Mock).mockImplementationOnce(() => ({
            runAsync: jest.fn().mockResolvedValue({
                selection_index: 0,
                selection_name: "MYSTERY PC",
            }),
            runHallOfFame: jest.fn().mockResolvedValue({ success: true }),
            runHallOfFameInteractive: jest.fn().mockResolvedValue({ success: true, interactive: true }),
            runHallOfFameInteractiveAsync: jest.fn().mockResolvedValue({ success: true, interactive: true }),
        }));

        await expect(pokemon_center_pc(gameState, { runner, overworld })).rejects.toThrow(
            "Missing ASM PC result text for selection 'MYSTERY PC'."
        );
    });
});

describe('BillPC', () => {
    it("should call pokemon_center_pc with the correct arguments", async () => {
        const gameState = createInitialGameState();
        const runner = createScriptRunnerStub({}) as unknown as ScriptRunner;
        const overworld = {
            script_runner: runner,
            event_manager: {},
        } as any;

        await BillPC(gameState, { overworld });

        expect(pcHelpers.pcHubEntries).toHaveBeenCalled();
    });
});

describe('hall_of_fame_terminal', () => {
    it('should run the Hall of Fame terminal and update runner state', async () => {
        const gameState = createInitialGameState();
        const runner = createScriptRunnerStub({
            last_value: null,
        }) as unknown as ScriptRunner;
        const overworld = {
            ui: {},
            dialogue: {},
            audio_engine: {},
            draw: jest.fn(),
        } as any;

        const result = await hall_of_fame_terminal(gameState, { runner, overworld });

        expect(result).toEqual({ success: true, interactive: true });
        expect(runner.last_value).toEqual({ hall_of_fame: { success: true, interactive: true } });
    });
});
