/**
 * Trade Manager
 *
 * Implements a strict, lockstep trade protocol over the LinkCableEmulator.
 *
 * Notes:
 * - We exchange fixed-size packets so both peers can use `sendBytes()` in sync.
 * - Pokemon payload is a Gen 2-style party mon struct (48 bytes) plus OT name (11 bytes).
 */

import type { Pokemon, PokemonSpecies } from '@pokecrystal/core/core/models/pokemon';
import { PokemonSchema, pokemonSpeciesDisplayName, toPokemon } from '@pokecrystal/core/core/models/pokemon';
import { loadAllItems, loadAllSpecies } from '@pokecrystal/core/core/data-loader';
import { getAsmMoveNameOrder } from '@pokecrystal/core/core/asm-move-names-loader';
import { StatusCondition } from '@pokecrystal/core/core/enums/battle';
import { MoveName } from '@pokecrystal/core/core/enums/move';
import { LinkCableEmulator } from './link-cable';

export type TradePhase =
  | 'idle'
  | 'connecting'
  | 'selecting'
  | 'confirming'
  | 'trading'
  | 'complete'
  | 'cancelled'
  | 'error';

export type TradeResult = {
  cancelled: boolean;
  receivedPokemon: Pokemon;
};

export type TradeOptions = {
  confirm: boolean;
};

export type TradeManagerOptions = {
  isHost: boolean;
  speciesById?: (id: number) => PokemonSpecies;
};

const READY = 0xa0;
const TRADE_START = 0xa1;
const TRADE_DONE = 0xa2;

const OT_NAME_BYTES = 11;
const POKEMON_STRUCT_BYTES = 48;
const TRADE_PACKET_BYTES = POKEMON_STRUCT_BYTES + OT_NAME_BYTES;
const SLP_MASK = 0b111;
const MON_STATUS_UNKNOWN_MASK = 0x80;
const MON_STATUS_PSN_MASK = 0x08;
const MON_STATUS_BRN_MASK = 0x10;
const MON_STATUS_FRZ_MASK = 0x20;
const MON_STATUS_PAR_MASK = 0x40;
const MON_STATUS_STATUS_MASK =
  MON_STATUS_PSN_MASK | MON_STATUS_BRN_MASK | MON_STATUS_FRZ_MASK | MON_STATUS_PAR_MASK;

type PartyMonStatus = {
  status?: StatusCondition | null;
  sleepTurns: number;
};

export class TradeManager {
  public phase: TradePhase = 'idle';

  private readonly link: LinkCableEmulator;
  private readonly isHost: boolean;
  private readonly speciesById: (id: number) => PokemonSpecies;
  private readonly moveOrder: readonly MoveName[];
  private readonly moveIdByName: Map<MoveName, number>;
  private readonly itemIdByName: Map<string, number>;
  private readonly itemNameById: Map<number, string>;

  constructor(link: LinkCableEmulator, options: TradeManagerOptions) {
    this.link = link;
    this.isHost = options.isHost;
    this.speciesById =
      options.speciesById ??
      TradeManager.buildSpeciesByIdResolver(loadAllSpecies());

    this.moveOrder = getAsmMoveNameOrder();
    this.moveIdByName = new Map(
      this.moveOrder.map((name, index) => [name, index + 1]),
    );
    const items = loadAllItems();
    this.itemIdByName = new Map<string, number>();
    this.itemNameById = new Map<number, string>();
    let itemId = 0;
    for (const item of items.values()) {
      itemId += 1;
      this.itemNameById.set(itemId, item.name);
      const canonical = TradeManager.canonicalizeItemName(item.name);
      if (canonical.length > 0) {
        this.itemIdByName.set(canonical, itemId);
      }
      if (item.script_name) {
        this.itemIdByName.set(item.script_name, itemId);
      }
      this.itemIdByName.set(item.name.toUpperCase(), itemId);
    }
  }

  async trade(localPokemon: Pokemon, options: TradeOptions): Promise<TradeResult> {
    if (this.phase !== 'idle') {
      throw new Error(`TradeManager is busy (phase=${this.phase})`);
    }

    const localPacket = this.serializeTradePacket(localPokemon);

    this.phase = 'connecting';
    const ok = await this.link.establishConnection();
    if (!ok) {
      this.phase = 'error';
      throw new Error('TradeManager: failed to establish link connection');
    }

    // Step 0: ready sync.
    await this.exchangeFixed([READY], 1, 'ready');

    // Step 1: exchange Pokemon payload (48 + 11).
    this.phase = 'selecting';
    const remotePacket = await this.exchangeFixed(localPacket, TRADE_PACKET_BYTES, 'select');
    const remotePokemon = this.deserializeTradePacket(remotePacket);

    // Step 2: confirm/cancel.
    this.phase = 'confirming';
    const localConfirm = options.confirm ? 0x01 : 0x00;
    const [remoteConfirm] = await this.exchangeFixed([localConfirm], 1, 'confirm');
    const cancelled = localConfirm === 0x00 || remoteConfirm === 0x00;
    if (cancelled) {
      this.phase = 'cancelled';
      return { cancelled: true, receivedPokemon: remotePokemon };
    }

    // Step 3: trade start barrier.
    this.phase = 'trading';
    await this.exchangeFixed([TRADE_START], 1, 'trade_start');

    // Synchronized animation delay (3 seconds).
    await new Promise((resolve) => setTimeout(resolve, 3000));

    // Step 4: done barrier.
    await this.exchangeFixed([TRADE_DONE], 1, 'trade_done');

    this.phase = 'complete';
    return { cancelled: false, receivedPokemon: remotePokemon };
  }

  private async exchangeFixed(
    bytes: number[] | Uint8Array,
    expectedLen: number,
    label: string,
  ): Promise<number[]> {
    const outgoing = Array.from(bytes);
    if (outgoing.length !== expectedLen) {
      throw new Error(
        `TradeManager(${label}): outgoing length mismatch: ${outgoing.length} != ${expectedLen}`,
      );
    }
    const incoming = await this.link.sendBytes(outgoing);
    if (incoming.length !== expectedLen) {
      throw new Error(
        `TradeManager(${label}): incoming length mismatch: ${incoming.length} != ${expectedLen}`,
      );
    }
    if (expectedLen === 1) {
      // Simple sync checks for 1-byte packets.
      if (label === 'ready' && incoming[0] !== READY) {
        throw new Error(`TradeManager: expected READY(0x${READY.toString(16)}) but got 0x${incoming[0].toString(16)}`);
      }
      if (label === 'trade_start' && incoming[0] !== TRADE_START) {
        throw new Error(`TradeManager: expected TRADE_START but got 0x${incoming[0].toString(16)}`);
      }
      if (label === 'trade_done' && incoming[0] !== TRADE_DONE) {
        throw new Error(`TradeManager: expected TRADE_DONE but got 0x${incoming[0].toString(16)}`);
      }
      if (label === 'confirm' && incoming[0] !== 0x00 && incoming[0] !== 0x01) {
        throw new Error(`TradeManager: invalid confirm byte 0x${incoming[0].toString(16)}`);
      }
    }
    return incoming;
  }

  private serializeTradePacket(pokemon: Pokemon): Uint8Array {
    const mon = this.serializePartyMon48(pokemon);
    const ot = TradeManager.encodeAsciiFixed(pokemon.original_trainer_name, OT_NAME_BYTES);
    const packet = new Uint8Array(TRADE_PACKET_BYTES);
    packet.set(mon, 0);
    packet.set(ot, POKEMON_STRUCT_BYTES);
    return packet;
  }

  private deserializeTradePacket(packet: number[]): Pokemon {
    if (packet.length !== TRADE_PACKET_BYTES) {
      throw new Error(`TradeManager: invalid packet length ${packet.length}`);
    }
    const bytes = Uint8Array.from(packet);
    const mon = bytes.slice(0, POKEMON_STRUCT_BYTES);
    const otName = TradeManager.decodeAsciiFixed(bytes.slice(POKEMON_STRUCT_BYTES));
    return this.deserializePartyMon48(mon, otName);
  }

  // ----------------------------------------------------------------------------
  // Gen 2-ish party mon struct (48 bytes)
  // ----------------------------------------------------------------------------

  private serializePartyMon48(pokemon: Pokemon): Uint8Array {
    const speciesId = pokemon.species.int_id;
    if (!Number.isInteger(speciesId) || speciesId <= 0 || speciesId > 255) {
      throw new Error(`TradeManager: invalid species int_id=${speciesId}`);
    }
    const heldItem = this.serializeHeldItem(pokemon.item);
    const status = this.serializePartyMonStatus(pokemon.status ?? null, pokemon.sleep_turns);

    const moveIds: number[] = [];
    const pp: number[] = [];
    for (let i = 0; i < 4; i++) {
      const move = pokemon.moves[i];
      if (!move) {
        moveIds.push(0);
        pp.push(0);
        continue;
      }
      const moveId = this.moveIdByName.get(move.name);
      if (!moveId) {
        throw new Error(`TradeManager: unknown move ${move.name}`);
      }
      if (moveId > 255) {
        throw new Error(`TradeManager: move id out of range for ${move.name}`);
      }
      moveIds.push(moveId);
      pp.push(TradeManager.clampByte(move.current_pp));
    }

    const out = new Uint8Array(POKEMON_STRUCT_BYTES);
    let o = 0;

    // BoxMon (32 bytes)
    out[o++] = speciesId & 0xff; // species
    out[o++] = heldItem;
    for (let i = 0; i < 4; i++) out[o++] = moveIds[i] & 0xff; // moves
    TradeManager.writeU16LE(out, o, pokemon.original_trainer_id);
    o += 2;
    TradeManager.writeU24LE(out, o, pokemon.experience);
    o += 3;
    TradeManager.writeU16LE(out, o, pokemon.hp_exp); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.attack_exp); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.defense_exp); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.speed_exp); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.special_exp); o += 2;

    // DVs packed (attack/def, speed/special)
    out[o++] = ((pokemon.dvs.attack & 0xf) << 4) | (pokemon.dvs.defense & 0xf);
    out[o++] = ((pokemon.dvs.speed & 0xf) << 4) | (pokemon.dvs.special & 0xf);

    for (let i = 0; i < 4; i++) out[o++] = pp[i] & 0xff; // pp
    out[o++] = TradeManager.clampByte(pokemon.happiness);
    out[o++] = pokemon.pokerus ? 0x01 : 0x00;
    out[o++] = 0x00; // caught data 1 (unknown)
    out[o++] = 0x00; // caught data 2 (unknown)
    out[o++] = TradeManager.clampByte(pokemon.level);

    // PartyMon extra (16 bytes)
    out[o++] = status;
    out[o++] = 0x00; // unused
    TradeManager.writeU16LE(out, o, pokemon.hp); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.max_hp); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.attack); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.defense); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.speed); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.special_attack); o += 2;
    TradeManager.writeU16LE(out, o, pokemon.special_defense); o += 2;

    if (o !== POKEMON_STRUCT_BYTES) {
      throw new Error(`TradeManager: party mon serialization length mismatch (${o})`);
    }
    return out;
  }

  private deserializePartyMon48(bytes: Uint8Array, otName: string): Pokemon {
    if (bytes.length !== POKEMON_STRUCT_BYTES) {
      throw new Error(`TradeManager: invalid party mon length ${bytes.length}`);
    }
    let o = 0;
    const speciesId = bytes[o++];
    const itemId = bytes[o++]; // currently unused
    let item: string | undefined;
    if (itemId !== 0) {
      const itemName = this.itemNameById.get(itemId);
      if (!itemName) {
        throw new Error(`TradeManager: received unknown held item id ${itemId}`);
      }
      item = itemName;
    }
    const moveIds = [bytes[o++], bytes[o++], bytes[o++], bytes[o++]];
    const otId = TradeManager.readU16LE(bytes, o); o += 2;
    const exp = TradeManager.readU24LE(bytes, o); o += 3;
    const hpExp = TradeManager.readU16LE(bytes, o); o += 2;
    const atkExp = TradeManager.readU16LE(bytes, o); o += 2;
    const defExp = TradeManager.readU16LE(bytes, o); o += 2;
    const spdExp = TradeManager.readU16LE(bytes, o); o += 2;
    const spcExp = TradeManager.readU16LE(bytes, o); o += 2;
    const dv1 = bytes[o++];
    const dv2 = bytes[o++];
    const pp = [bytes[o++], bytes[o++], bytes[o++], bytes[o++]];
    const happiness = bytes[o++];
    const pokerus = bytes[o++] !== 0;
    o += 2; // caught data
    const level = bytes[o++];
    const statusByte = bytes[o++];
    const { status, sleepTurns } = this.deserializePartyMonStatus(statusByte);
    o++; // unused
    const hp = TradeManager.readU16LE(bytes, o); o += 2;
    const maxHp = TradeManager.readU16LE(bytes, o); o += 2;
    const attack = TradeManager.readU16LE(bytes, o); o += 2;
    const defense = TradeManager.readU16LE(bytes, o); o += 2;
    const speed = TradeManager.readU16LE(bytes, o); o += 2;
    const specialAttack = TradeManager.readU16LE(bytes, o); o += 2;
    const specialDefense = TradeManager.readU16LE(bytes, o); o += 2;

    if (o !== POKEMON_STRUCT_BYTES) {
      throw new Error(`TradeManager: party mon parse length mismatch (${o})`);
    }

    const species = this.speciesById(speciesId);
    const moves = moveIds
      .map((id, idx) => {
        if (id === 0) return null;
        const moveName = this.moveOrder[id - 1];
        if (!moveName) {
          throw new Error(`TradeManager: unknown move id ${id}`);
        }
        return { name: moveName, current_pp: pp[idx] };
      })
      .filter(Boolean) as Array<{ name: MoveName; current_pp: number }>;

    const dvs = {
      attack: (dv1 >> 4) & 0xf,
      defense: dv1 & 0xf,
      speed: (dv2 >> 4) & 0xf,
      special: dv2 & 0xf,
      hp: 0,
    };

    const data = PokemonSchema.parse({
      species,
      nickname: pokemonSpeciesDisplayName(species),
      moves,
      level,
      hp,
      max_hp: maxHp,
      dvs,
      attack,
      defense,
      speed,
      special_attack: specialAttack,
      special_defense: specialDefense,
      item,
      status,
      sleep_turns: sleepTurns,
      original_trainer_name: otName || 'TRAINER',
      original_trainer_id: otId,
      experience: exp,
      happiness,
      pokerus,
      hp_exp: hpExp,
      attack_exp: atkExp,
      defense_exp: defExp,
      speed_exp: spdExp,
      special_exp: spcExp,
    });
    return toPokemon(data);
  }

  private serializeHeldItem(item?: string | null): number {
    const normalized = item ? TradeManager.canonicalizeItemName(item) : "";
    if (!normalized) {
      return 0x00;
    }
    const itemId = this.itemIdByName.get(normalized);
    if (!itemId) {
      throw new Error(`TradeManager: unknown held item '${item}'`);
    }
    if (itemId > 0xff) {
      throw new Error(`TradeManager: held item id ${itemId} out of range`);
    }
    return itemId;
  }

  private serializePartyMonStatus(status: StatusCondition | null, sleepTurns: number): number {
    if (!status || status === StatusCondition.NONE) {
      if (sleepTurns !== 0) {
        throw new Error('TradeManager: sleep_turns requires SLEEP status');
      }
      return 0x00;
    }
    if (sleepTurns < 0) {
      throw new Error('TradeManager: sleep turns must be non-negative');
    }
    switch (status) {
      case StatusCondition.POISON:
        if (sleepTurns !== 0) {
          throw new Error('TradeManager: non-sleep statuses must not have sleep_turns');
        }
        return MON_STATUS_PSN_MASK;
      case StatusCondition.BURN:
        if (sleepTurns !== 0) {
          throw new Error('TradeManager: non-sleep statuses must not have sleep_turns');
        }
        return MON_STATUS_BRN_MASK;
      case StatusCondition.FREEZE:
        if (sleepTurns !== 0) {
          throw new Error('TradeManager: non-sleep statuses must not have sleep_turns');
        }
        return MON_STATUS_FRZ_MASK;
      case StatusCondition.PARALYSIS:
        if (sleepTurns !== 0) {
          throw new Error('TradeManager: non-sleep statuses must not have sleep_turns');
        }
        return MON_STATUS_PAR_MASK;
      case StatusCondition.SLEEP:
        if (!Number.isInteger(sleepTurns) || sleepTurns < 1 || sleepTurns > SLP_MASK) {
          throw new Error('TradeManager: sleep_turns for SLEEP must be 1..7');
        }
        return sleepTurns & SLP_MASK;
      case StatusCondition.CONFUSION:
      default:
        throw new Error(`TradeManager: unsupported status '${status}' for trading`);
    }
  }

  private deserializePartyMonStatus(status: number): PartyMonStatus {
    if (!Number.isInteger(status) || status < 0 || status > 0xff) {
      throw new Error(`TradeManager: invalid MON_STATUS ${status}`);
    }
    if (status & MON_STATUS_UNKNOWN_MASK) {
      throw new Error(`TradeManager: unsupported MON_STATUS bits ${status}`);
    }

    const sleepTurns = status & SLP_MASK;
    const statusFlags = status & MON_STATUS_STATUS_MASK;

    if (statusFlags !== 0 && sleepTurns !== 0) {
      throw new Error('TradeManager: MON_STATUS has conflicting sleep and status flags');
    }
    if (statusFlags && (statusFlags & (statusFlags - 1)) !== 0) {
      throw new Error('TradeManager: MON_STATUS has multiple status flags');
    }

    if (statusFlags & MON_STATUS_PSN_MASK) {
      return { status: StatusCondition.POISON, sleepTurns: 0 };
    }
    if (statusFlags & MON_STATUS_BRN_MASK) {
      return { status: StatusCondition.BURN, sleepTurns: 0 };
    }
    if (statusFlags & MON_STATUS_FRZ_MASK) {
      return { status: StatusCondition.FREEZE, sleepTurns: 0 };
    }
    if (statusFlags & MON_STATUS_PAR_MASK) {
      return { status: StatusCondition.PARALYSIS, sleepTurns: 0 };
    }
    if (sleepTurns === 0) {
      return { status: StatusCondition.NONE, sleepTurns: 0 };
    }
    return { status: StatusCondition.SLEEP, sleepTurns };
  }

  // ----------------------------------------------------------------------------
  // Helpers
  // ----------------------------------------------------------------------------

  static buildSpeciesByIdResolver(speciesMap: Map<string, PokemonSpecies>): (id: number) => PokemonSpecies {
    const byId = new Map<number, PokemonSpecies>();
    for (const species of speciesMap.values()) {
      byId.set(species.int_id, species);
    }
    return (id: number) => {
      const species = byId.get(id);
      if (!species) {
        throw new Error(`TradeManager: unknown species id ${id}`);
      }
      return species;
    };
  }

  static encodeAsciiFixed(value: string, length: number): Uint8Array {
    if (typeof value !== 'string') {
      throw new Error('OT name must be a string');
    }
    const trimmed = value.trim();
    const bytes = new Uint8Array(length);
    for (let i = 0; i < length; i++) bytes[i] = 0;
    const max = Math.min(length, trimmed.length);
    for (let i = 0; i < max; i++) {
      const code = trimmed.charCodeAt(i);
      if (code < 0x20 || code > 0x7e) {
        throw new Error('OT name must be ASCII printable');
      }
      bytes[i] = code & 0xff;
    }
    return bytes;
  }

  static decodeAsciiFixed(bytes: Uint8Array): string {
    let end = bytes.length;
    while (end > 0 && bytes[end - 1] === 0) end--;
    let out = '';
    for (let i = 0; i < end; i++) {
      const code = bytes[i];
      if (code < 0x20 || code > 0x7e) {
        throw new Error('OT name decode: non-ascii byte encountered');
      }
      out += String.fromCharCode(code);
    }
    return out;
  }

  private static canonicalizeItemName(item: string): string {
    return String(item)
      .trim()
      .toUpperCase()
      .replace(/[^0-9A-Z]+/g, '_')
      .replace(/^_+|_+$/g, '');
  }

  static clampByte(value: number): number {
    if (!Number.isFinite(value)) return 0;
    return Math.max(0, Math.min(0xff, Math.trunc(value)));
  }

  static writeU16LE(buf: Uint8Array, offset: number, value: number): void {
    const v = Math.max(0, Math.min(0xffff, Math.trunc(value)));
    buf[offset] = v & 0xff;
    buf[offset + 1] = (v >> 8) & 0xff;
  }

  static readU16LE(buf: Uint8Array, offset: number): number {
    return (buf[offset] | (buf[offset + 1] << 8)) >>> 0;
  }

  static writeU24LE(buf: Uint8Array, offset: number, value: number): void {
    const v = Math.max(0, Math.min(0xffffff, Math.trunc(value)));
    buf[offset] = v & 0xff;
    buf[offset + 1] = (v >> 8) & 0xff;
    buf[offset + 2] = (v >> 16) & 0xff;
  }

  static readU24LE(buf: Uint8Array, offset: number): number {
    return (buf[offset] | (buf[offset + 1] << 8) | (buf[offset + 2] << 16)) >>> 0;
  }
}
