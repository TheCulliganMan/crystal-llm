import { Pokemon } from '../../core/models';

export class BattleEvolutionCutscene {
  public is_finished = false;
  public was_cancelled = false;

  constructor(
    _ui: unknown,
    _audio_engine: unknown,
    _old_species_id: string,
    _new_species_id: string,
    _pokemon: Pokemon
  ) {}

  update(_surface: unknown, _options?: { dialogue_waiting?: boolean; cancel_requested?: boolean }): boolean {
    this.is_finished = true;
    return true;
  }
}
