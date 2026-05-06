import { DataLoader } from '@pokecrystal/core/core/data-loader';
import { MoveName } from '@pokecrystal/core/core/enums';
import { readFileSync } from 'fs';
import path from 'path';
import { getDataDir } from '@pokecrystal/core/core/paths';

jest.mock('fs', () => ({
    ...jest.requireActual('fs'),
    readFileSync: jest.fn(),
}));

const mockedReadFileSync = readFileSync as jest.Mock;
const DATA_DIR = getDataDir();

describe('DataLoader', () => {
    let dataLoader: DataLoader;

    beforeEach(() => {
        dataLoader = new DataLoader();
        mockedReadFileSync.mockClear();
    });

    describe('Trainer data loading', () => {
        it('should load trainer data', () => {
            const mockTrainerData = [
                {
                    "name": "YOUNGSTER_JOEY",
                    "party": [
                        {
                            "species": {
                                "id": "RATTATA",
                                "int_id": 25,
                                "base_stats": { "hp": 30, "attack": 56, "defense": 35, "speed": 72, "special_attack": 25, "special_defense": 35 },
                                "type1": "NORMAL",
                                "type2": "NORMAL",
                                "catch_rate": 255,
                                "base_exp": 57,
                                "gender_ratio": 127,
                                "unknown1": 100,
                                "step_cycles_to_hatch": 15,
                                "unknown2": 5,
                                "growth_rate": "GROWTH_MEDIUM_FAST",
                                "egg_group1": "EGG_GROUND",
                                "egg_group2": "EGG_GROUND"
                            },
                            "nickname": "RATTATA",
                            "level": 4,
                            "hp": 20,
                            "max_hp": 20,
                            "original_trainer_name": "JOEY",
                            "original_trainer_id": 12345,
                            "experience": 100,
                            "happiness": 70
                        }
                    ],
                    "win_quote": "I won!",
                    "lose_quote": "I lost!"
                }
            ];

            mockedReadFileSync.mockImplementation((filePath) => {
                if (filePath === path.join(DATA_DIR, 'trainers.json')) {
                    return JSON.stringify(mockTrainerData);
                }
                return '{}';
            });

            dataLoader.load_trainer_data();

            const trainer = dataLoader.get_trainer('YOUNGSTER_JOEY');
            expect(trainer).toBeDefined();
            expect(trainer?.name).toBe('YOUNGSTER_JOEY');
        });
    });

    describe('PC event injection', () => {
        it('should inject a PC background event for pokecenter maps', () => {
            const mapName = 'GoldenrodPokecenter1F';
            const mockMapDimensions = {
                "GOLDENROD_POKECENTER_1F": {
                    width: 10,
                    height: 10
                }
            };
            const mockNpcData = {
                [mapName]: []
            };
            const mockMapAttributes = {
                [mapName]: {
                    "tileset_name": "johto",
                    "border_block": 1,
                    "width": 10,
                    "height": 10,
                    "location": "Goldenrod City",
                    "music": "MUSIC_POKECENTER",
                    "time_of_day": "DAY",
                    "fishing_group": "FISHING_NONE"
                }
            };

            const mockMapEvents = {
                [`${mapName}_MapEvents`]: [
                    { command: 'def_bg_events', args: '' },
                ],
            };

            const storyEventsContent = {
                [mapName]: mockMapEvents,
            };

            mockedReadFileSync.mockImplementation((filePath) => {
                if (filePath === path.join(DATA_DIR, 'map_dimensions.json')) {
                    return JSON.stringify(mockMapDimensions);
                }
                if (filePath === path.join(DATA_DIR, 'npcs.json')) {
                    return JSON.stringify(mockNpcData);
                }
                if (filePath === path.join(DATA_DIR, 'map_attributes.json')) {
                    return JSON.stringify(mockMapAttributes);
                }
                if (filePath === path.join(DATA_DIR, 'story_events.json')) {
                    return JSON.stringify(storyEventsContent);
                }
                return '{}';
            });


            dataLoader.ensure_overworld_data({ map_name: mapName });

            const mapEvents = dataLoader.map_events.get(mapName);
            expect(mapEvents).toBeDefined();

            const pcEvent = mapEvents?.bg_events.find(
                (event) => event.script.toUpperCase() === 'PCSCRIPT'
            );

            expect(pcEvent).toBeDefined();
            expect(pcEvent?.x).toBe(9);
            expect(pcEvent?.y).toBe(1);
            expect(pcEvent?.event_type).toBe('BGEVENT_UP');
        });
    });

    describe('ensure_battle_data', () => {
        it('should load move metadata when missing', () => {
            dataLoader.moveData.clear();

            const movePayload = {
                TACKLE: {
                    name: 'TACKLE',
                    type: 'NORMAL',
                    power: 35,
                    accuracy: 95,
                    pp: 35,
                },
            };

            mockedReadFileSync.mockImplementation((filePath) => {
                if (filePath === path.join(DATA_DIR, 'moves_data.json')) {
                    return JSON.stringify(movePayload);
                }
                if (filePath === path.join(DATA_DIR, 'items.json')) {
                    return JSON.stringify([]);
                }
                if (filePath === path.join(DATA_DIR, 'pokemon_data.json')) {
                    return JSON.stringify({});
                }
                return '{}';
            });

            dataLoader.ensure_battle_data();

            expect(dataLoader.moveData.get(MoveName.TACKLE)).toBeDefined();
        });
    });
});
