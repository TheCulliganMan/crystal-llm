import { LinkCableEmulator } from './link-cable';
import { TradeManager } from './trade-manager';
import { loadAllSpecies } from '@pokecrystal/core/core/data-loader';
import { PokemonSchema, pokemonSpeciesDisplayName, toPokemon, type Pokemon } from '@pokecrystal/core/core/models/pokemon';
import { MoveName } from '@pokecrystal/core/core/enums/move';
import { StatusCondition } from '@pokecrystal/core/core/enums/battle';

class MockWebRTCConnection {
  private callbacks: Array<(msg: any) => void> = [];
  public peer: MockWebRTCConnection | null = null;

  onData(cb: (msg: any) => void): void {
    this.callbacks.push(cb);
  }

  send(msg: any): void {
    this.peer?.callbacks.forEach((cb) => cb(msg));
  }

  destroy(): void {}
}

function makePair(): [MockWebRTCConnection, MockWebRTCConnection] {
  const a = new MockWebRTCConnection();
  const b = new MockWebRTCConnection();
  a.peer = b;
  b.peer = a;
  return [a, b];
}

function samplePokemon(speciesKey: string, overrides?: Partial<Pokemon>): Pokemon {
  const species = loadAllSpecies().get(speciesKey);
  if (!species) {
    throw new Error(`missing species: ${speciesKey}`);
  }
  const data = PokemonSchema.parse({
    species,
    nickname: pokemonSpeciesDisplayName(species),
    moves: [
      { name: MoveName.POUND, current_pp: 35 },
      { name: MoveName.TACKLE, current_pp: 35 },
      { name: MoveName.GROWL, current_pp: 40 },
      { name: MoveName.TAIL_WHIP, current_pp: 30 },
    ],
    level: 12,
    hp: 39,
    max_hp: 39,
    dvs: { attack: 1, defense: 2, speed: 3, special: 4, hp: 0 },
    attack: 20,
    defense: 18,
    speed: 16,
    special_attack: 15,
    special_defense: 15,
    original_trainer_name: 'ALICE',
    original_trainer_id: 0x1234,
    experience: 12345,
    happiness: 70,
    hp_exp: 1,
    attack_exp: 2,
    defense_exp: 3,
    speed_exp: 4,
    special_exp: 5,
    pokerus: false,
    ...(overrides ?? {}),
  });
  return toPokemon(data);
}

describe('TradeManager', () => {
  test('trades swap pokemon payloads and preserve OT info', async () => {
    const [a, b] = makePair();
    const hostLink = new LinkCableEmulator(a as any, true);
    const clientLink = new LinkCableEmulator(b as any, false);

    const host = new TradeManager(hostLink, { isHost: true });
    const client = new TradeManager(clientLink, { isHost: false });

    const hostMon = samplePokemon('PIKACHU', { original_trainer_name: 'ALICE', original_trainer_id: 0x1111 });
    const clientMon = samplePokemon('EEVEE', { original_trainer_name: 'BOB', original_trainer_id: 0x2222 });

    const [hostResult, clientResult] = await Promise.all([
      host.trade(hostMon, { confirm: true }),
      client.trade(clientMon, { confirm: true }),
    ]);

    expect(hostResult.cancelled).toBe(false);
    expect(clientResult.cancelled).toBe(false);

    expect(hostResult.receivedPokemon.species.id).toBe(clientMon.species.id);
    expect(hostResult.receivedPokemon.original_trainer_name).toBe('BOB');
    expect(hostResult.receivedPokemon.original_trainer_id).toBe(0x2222);

    expect(clientResult.receivedPokemon.species.id).toBe(hostMon.species.id);
    expect(clientResult.receivedPokemon.original_trainer_name).toBe('ALICE');
    expect(clientResult.receivedPokemon.original_trainer_id).toBe(0x1111);
  });

  test('cancelled if either side does not confirm', async () => {
    const [a, b] = makePair();
    const hostLink = new LinkCableEmulator(a as any, true);
    const clientLink = new LinkCableEmulator(b as any, false);

    const host = new TradeManager(hostLink, { isHost: true });
    const client = new TradeManager(clientLink, { isHost: false });

    const hostMon = samplePokemon('PIKACHU');
    const clientMon = samplePokemon('EEVEE');

    const [hostResult, clientResult] = await Promise.all([
      host.trade(hostMon, { confirm: false }),
      client.trade(clientMon, { confirm: true }),
    ]);

    expect(hostResult.cancelled).toBe(true);
    expect(clientResult.cancelled).toBe(true);
  });

  test('serializes held item and status values', async () => {
    const [a, b] = makePair();
    const hostLink = new LinkCableEmulator(a as any, true);
    const clientLink = new LinkCableEmulator(b as any, false);

    const host = new TradeManager(hostLink, { isHost: true });
    const client = new TradeManager(clientLink, { isHost: false });

    const hostMon = samplePokemon('PIKACHU', {
      item: 'BERRY' as any,
      status: StatusCondition.SLEEP,
      sleep_turns: 3,
    });
    const clientMon = samplePokemon('EEVEE', {
      item: 'SUPER POTION',
      status: StatusCondition.POISON,
    });

    const [hostResult, clientResult] = await Promise.all([
      host.trade(hostMon, { confirm: true }),
      client.trade(clientMon, { confirm: true }),
    ]);

    expect(hostResult.cancelled).toBe(false);
    expect(hostResult.receivedPokemon.item).toBe('SUPER POTION');
    expect(hostResult.receivedPokemon.status).toBe(StatusCondition.POISON);
    expect(hostResult.receivedPokemon.sleep_turns).toBe(0);

    expect(clientResult.cancelled).toBe(false);
    expect(clientResult.receivedPokemon.item).toBe('BERRY');
    expect(clientResult.receivedPokemon.status).toBe(StatusCondition.SLEEP);
    expect(clientResult.receivedPokemon.sleep_turns).toBe(3);
  });

  test('throws for unknown held item names', async () => {
    const [a, b] = makePair();
    const hostLink = new LinkCableEmulator(a as any, true);
    const clientLink = new LinkCableEmulator(b as any, false);

    const host = new TradeManager(hostLink, { isHost: true });
    void new TradeManager(clientLink, { isHost: false });

    const hostMon = samplePokemon('PIKACHU', { item: 'NOT_A_POKEMON_ITEM' as any });
    await expect(host.trade(hostMon, { confirm: true })).rejects.toThrow(/unknown held item/i);
  });

  test('throws for unsupported status values', async () => {
    const [a, b] = makePair();
    const hostLink = new LinkCableEmulator(a as any, true);
    const clientLink = new LinkCableEmulator(b as any, false);

    const host = new TradeManager(hostLink, { isHost: true });
    void new TradeManager(clientLink, { isHost: false });

    const hostMon = samplePokemon('PIKACHU', {
      status: StatusCondition.CONFUSION,
      sleep_turns: 0,
    });
    await expect(host.trade(hostMon, { confirm: true })).rejects.toThrow(/unsupported status/i);
  });
});
