import type { Options, Pokemon, PokemonSpecies } from "@pokecrystal/core/core/models";
import { getSpecies as getCanonicalSpecies } from "@pokecrystal/core/core/data-loader";
import { TextSpeed } from "@pokecrystal/core/core/enums/ui-enums";
import { clearJoypad } from "@pokecrystal/core/core/home";
import type { GameState } from "@pokecrystal/core/core/state";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import { DudeAutoInputController, AUTO_INPUT } from "@pokecrystal/core/engine/battle/auto-input";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { Event, EventManager, StartBattleEvent, type EventPayload } from "@pokecrystal/core/engine/events/events";

// ASM mapping: engine/overworld/scripting.asm::Script_catchtutorial and
// engine/events/catch_tutorial.asm::CatchTutorial (name/bag swap + auto input).

const DEFAULT_BATTLE_TYPE = "BATTLETYPE_NORMAL";
const TEXT_SPEED_MASK = 0b111;
const TEXT_SPEED_MID = 0x03;
type PokemonSpeciesLookup = {
  pokemonData?: Map<string, PokemonSpecies>;
  pokemon_data?: Record<string, PokemonSpecies>;
  get_pokemon_species?: (name: string) => PokemonSpecies | undefined;
  getPokemonSpecies?: (name: string) => PokemonSpecies | undefined;
  getSpecies?: (name: string) => PokemonSpecies | undefined;
};
type TutorialDataLoader = DataLoader | PokemonSpeciesLookup;

export class CatchTutorialRunner {
  constructor(
    private gameState: GameState,
    private eventManager: EventManager,
    private dataLoader: TutorialDataLoader,
  ) {}

  public run(options: {
    wild_species: string;
    wild_level: number;
    battle_type: string;
    on_complete?: (() => void) | null;
  }): void {
    const { wild_species, wild_level, battle_type, on_complete } = options;

    if (wild_level <= 0) {
      throw new Error("Catching tutorial requires a positive wild Pokemon level.");
    }

    const speciesKey = wild_species.trim().toUpperCase();
    const wildSpecies = this.resolveSpecies(speciesKey);
    if (!wildSpecies) {
      throw new Error(`Unknown wild species '${wild_species}' for catching tutorial`);
    }
    const party = (this.gameState.sram.party?.pokemon ?? []).filter(
      (pokemon): pokemon is Pokemon => pokemon !== null,
    );
    if (!party.length) {
      throw new Error("Catching tutorial requires at least one Pokemon in party.");
    }
    const playerMon = party[0];

    const previousName = this.gameState.sram.player_name;
    const previousOptions = this.copyOptions();
    const wram = this.gameState.wram;
    const previousWOptions = wram.wOptions;
    const previousWild = { ...wram.wild_pokemon };
    const previousTrainerClass = wram.other_trainer_class;
    const previousTrainerId = wram.other_trainer_id;
    const previousTrainer = wram.other_trainer;
    const previousTrainerParty = [...(wram.other_trainer_party ?? [])];
    const autoInput = new DudeAutoInputController();

    const wildEntry = { species: speciesKey, level: wild_level };

    let success = false;
    let battleDispatched = false;
    let listenerRegistered = false;
    let stateRestored = false;

    const clearTrainerContext = (): void => {
      wram.other_trainer_class = "";
      wram.other_trainer_id = "";
      wram.other_trainer = undefined;
      wram.other_trainer_party = [];
    };

    const restoreState = (): void => {
      if (stateRestored) {
        return;
      }
      stateRestored = true;
      this.gameState.sram.player_name = wram.wMomsName || previousName;
      this.gameState.sram.options = previousOptions;
      wram.wOptions = previousWOptions;
      wram.wild_pokemon = previousWild;
      wram.battle_type = DEFAULT_BATTLE_TYPE;
      wram.wInputType = 0;
      if (battleDispatched) {
        clearTrainerContext();
      } else {
        wram.other_trainer_class = previousTrainerClass;
        wram.other_trainer_id = previousTrainerId;
        wram.other_trainer = previousTrainer;
        wram.other_trainer_party = previousTrainerParty;
      }
      autoInput.stop();
      if (listenerRegistered) {
        this.eventManager.off("battle_complete", notifyOnComplete);
        listenerRegistered = false;
      }
    };

    const notifyOnComplete = (_event: Event<EventPayload>, _state: GameState): void => {
      restoreState();
      if (on_complete) {
        on_complete();
      }
    };

    try {
      wram.wMomsName = previousName;
      this.gameState.sram.player_name = "DUDE";
      this.gameState.sram.options = this.withMidTextSpeed(previousOptions);
      wram.wOptions = this.withMidTextSpeedMask(previousWOptions);
      this.applyTutorialBag();
      clearJoypad(this.gameState);
      wram.battle_type = battle_type;
      wram.wild_pokemon = wildEntry;
      wram.wInputType = AUTO_INPUT;
      clearTrainerContext();

      const wildMon = createPokemon(this.gameState, wildSpecies, wild_level);
      wildMon.original_trainer_name = "WILD";
      wildMon.original_trainer_id = 0;

      const startEvent = new StartBattleEvent({
        player_pokemon: playerMon,
        enemy_pokemon: wildMon,
        player_party: party,
        enemy_party: [wildMon],
        auto_input: autoInput,
      });
      this.eventManager.on("battle_complete", notifyOnComplete);
      listenerRegistered = true;
      battleDispatched = true;
      this.eventManager.dispatch(startEvent);

      success = true;
    } finally {
      if (!success) {
        restoreState();
      }
    }
  }

  private copyOptions(): Options {
    const options = this.gameState.sram.options;
    if (typeof structuredClone === "function") {
      return structuredClone(options);
    }
    return JSON.parse(JSON.stringify(options)) as Options;
  }

  private withMidTextSpeed(original: Options): Options {
    const clone = typeof structuredClone === "function"
      ? structuredClone(original)
      : (JSON.parse(JSON.stringify(original)) as Options);
    clone.text_speed = TextSpeed.MID;
    return clone;
  }

  private withMidTextSpeedMask(original: number): number {
    return (original & ~TEXT_SPEED_MASK) | TEXT_SPEED_MID;
  }

  private applyTutorialBag(): void {
    const wram = this.gameState.wram;
    wram.wDudeItems = { POTION: 1 };
    wram.wDudeKeyItems = {};
    wram.wDudeBalls = { POKE_BALL: 1 };
  }

  private resolveSpecies(name: string): PokemonSpecies | undefined {
    const loader = this.dataLoader as PokemonSpeciesLookup;
    if (loader.pokemonData instanceof Map) {
      const fromMap = loader.pokemonData.get(name);
      if (fromMap) {
        return fromMap;
      }
    }
    if (loader.pokemon_data && typeof loader.pokemon_data === "object") {
      const fromRecord = loader.pokemon_data[name];
      if (fromRecord) {
        return fromRecord;
      }
    }
    const getter =
      loader.get_pokemon_species ??
      loader.getPokemonSpecies ??
      loader.getSpecies;
    if (typeof getter === "function") {
      const fromGetter = getter.call(loader, name);
      if (fromGetter) {
        return fromGetter;
      }
    }
    return getCanonicalSpecies(name);
  }
}
