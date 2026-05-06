import { GameState } from "@pokecrystal/core/core/state";
import { countPokedexEntries } from "@pokecrystal/core/core/pokedex";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { MAX_MYSTERY_GIFT_PARTNERS } from "@pokecrystal/core/core/memory/sram";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";

export const POKEMON_PIKACHU_2_VERSION = "POKEMON_PIKACHU_2";
export const RESERVED_GAME_VERSION = "RESERVED";

export const PRESS_TO_LINK_TEXT =
  "Press A to\nlink IR-Device\nPress B to\ncancel it.";
export const CANCELED_TEXT = "The link has been\ncancelled.";
export const COMMUNICATION_ERROR_TEXT = "Communication\nerror.";
export const RETRIEVE_AT_CENTER_TEXT = "Must retrieve GIFT\nat POKÉMON CENTER.";
export const FRIEND_NOT_READY_TEXT = "Your friend isn't\nready.";
export const FIVE_A_DAY_TEXT = "Sorry--only five\nGIFTS a day.";
export const ONE_A_DAY_TEXT = "Sorry. One GIFT\na day per person.";

export class MysteryGiftError extends Error {}
export class MysteryGiftCanceledError extends MysteryGiftError {}
export class MysteryGiftCommunicationError extends MysteryGiftError {}

export type MysteryGiftPlayerPayload = {
  trainerId: number;
  name: string;
  dexCaught: number;
  backupItem: string | null;
  gameVersion?: string;
};

export type MysteryGiftPartnerPayload = {
  gameVersion: string;
  partnerId: number;
  name: string;
  dexCaught?: number;
  sentDecoration?: boolean;
  decorationId?: number | null;
  decorationName?: string | null;
  itemName?: string | null;
  backupItem?: string | null;
  trainerData?: Record<string, unknown> | null;
};

export enum MysteryGiftOutcomeStatus {
  ITEM = "item",
  DECORATION = "decoration",
  PENDING_REWARD = "pending_reward",
  FRIEND_NOT_READY = "friend_not_ready",
  DAILY_LIMIT = "daily_limit",
  REPEAT_PARTNER = "repeat_partner",
  CANCELED = "canceled",
  ERROR = "error",
}

export type MysteryGiftOutcome = {
  status: MysteryGiftOutcomeStatus;
  message: string;
  partnerName?: string | null;
  rewardItem?: string | null;
  rewardItemDisplay?: string | null;
  rewardDecoration?: number | null;
  decorationName?: string | null;
};

export interface MysteryGiftAdapter {
  exchange(payload: MysteryGiftPlayerPayload): MysteryGiftPartnerPayload;
}

export class UnavailableMysteryGiftAdapter implements MysteryGiftAdapter {
  exchange(_payload: MysteryGiftPlayerPayload): MysteryGiftPartnerPayload {
    throw new MysteryGiftCommunicationError(
      "Mystery Gift hardware emulation is not available."
    );
  }
}

export enum MysteryGiftState {
  INIT = "init",
  STAGING = "staging",
  EXCHANGE = "exchange",
  REWARD = "reward",
  COMPLETE = "complete",
}

const hasPendingItem = (saveData: GameState["sram"]["mystery_gift"]): boolean => {
  return Boolean(saveData.stored_item);
};

const registerPartner = (
  saveData: GameState["sram"]["mystery_gift"],
  partnerId: number
): void => {
  const normalized = partnerId & 0xffff;
  if (saveData.daily_partner_ids.includes(normalized)) {
    return;
  }
  if (saveData.daily_partner_ids.length >= MAX_MYSTERY_GIFT_PARTNERS) {
    return;
  }
  saveData.daily_partner_ids.push(normalized);
  saveData.recent_partner_id = normalized;
};

const hasReachedDailyLimit = (
  saveData: GameState["sram"]["mystery_gift"]
): boolean => {
  return saveData.daily_partner_ids.length >= MAX_MYSTERY_GIFT_PARTNERS;
};

const hasReceivedFromPartner = (
  saveData: GameState["sram"]["mystery_gift"],
  partnerId: number
): boolean => {
  return saveData.daily_partner_ids.includes(partnerId);
};

const recordDecoration = (
  saveData: GameState["sram"]["mystery_gift"],
  decorationId: number
): boolean => {
  const owned = new Set(saveData.decorations_received || []);
  if (owned.has(decorationId)) {
    return true;
  }
  owned.add(decorationId);
  saveData.decorations_received = Array.from(owned);
  return false;
};

export class MysteryGiftStateMachine {
  public state = MysteryGiftState.INIT;
  public lastOutcome: MysteryGiftOutcome | null = null;
  private readonly itemSystem: ItemSystem;

  constructor(
    private readonly gameState: GameState,
    private readonly adapter: MysteryGiftAdapter = new UnavailableMysteryGiftAdapter(),
    dataLoader: DataLoader | null = null
  ) {
    this.itemSystem = new ItemSystem(gameState, dataLoader ?? undefined);
  }

  reset(): void {
    this.state = MysteryGiftState.INIT;
    this.lastOutcome = null;
  }

  performExchange(): MysteryGiftOutcome {
    this.state = MysteryGiftState.STAGING;
    const playerPayload = this.stagePlayerPayload();
    this.state = MysteryGiftState.EXCHANGE;

    let partnerPayload: MysteryGiftPartnerPayload;
    try {
      partnerPayload = this.adapter.exchange(playerPayload);
    } catch (error) {
      if (error instanceof MysteryGiftCanceledError) {
        this.state = MysteryGiftState.COMPLETE;
        const outcome: MysteryGiftOutcome = {
          status: MysteryGiftOutcomeStatus.CANCELED,
          message: CANCELED_TEXT,
        };
        this.lastOutcome = outcome;
        return outcome;
      }
      if (error instanceof MysteryGiftCommunicationError) {
        this.state = MysteryGiftState.INIT;
        const outcome: MysteryGiftOutcome = {
          status: MysteryGiftOutcomeStatus.ERROR,
          message: COMMUNICATION_ERROR_TEXT,
        };
        this.lastOutcome = outcome;
        return outcome;
      }
      throw error;
    }

    this.state = MysteryGiftState.REWARD;
    const outcome = this.applyPartnerPayload(playerPayload, partnerPayload);
    this.state = MysteryGiftState.COMPLETE;
    this.lastOutcome = outcome;
    return outcome;
  }

  private stagePlayerPayload(): MysteryGiftPlayerPayload {
    const playerName = this.gameState.sram.player_name || "PLAYER";
    const dexCaught = countPokedexEntries(this.gameState.sram.pokedex_owned);
    return {
      trainerId: this.gameState.sram.player_id,
      name: playerName,
      dexCaught,
      backupItem: this.gameState.sram.mystery_gift.stored_item,
      gameVersion: "CRYSTAL",
    };
  }

  private applyPartnerPayload(
    playerPayload: MysteryGiftPlayerPayload,
    partnerPayload: MysteryGiftPartnerPayload
  ): MysteryGiftOutcome {
    const skipChecks = partnerPayload.gameVersion === POKEMON_PIKACHU_2_VERSION;
    const partnerId = partnerPayload.partnerId & 0xffff;
    const saveData = this.gameState.sram.mystery_gift;

    if (!skipChecks) {
      if (hasReachedDailyLimit(saveData)) {
        return {
          status: MysteryGiftOutcomeStatus.DAILY_LIMIT,
          message: FIVE_A_DAY_TEXT,
        };
      }
      if (hasReceivedFromPartner(saveData, partnerId)) {
        return {
          status: MysteryGiftOutcomeStatus.REPEAT_PARTNER,
          message: ONE_A_DAY_TEXT,
        };
      }
    }

    if (hasPendingItem(saveData)) {
      return {
        status: MysteryGiftOutcomeStatus.PENDING_REWARD,
        message: RETRIEVE_AT_CENTER_TEXT,
      };
    }

    if (partnerPayload.backupItem) {
      return {
        status: MysteryGiftOutcomeStatus.FRIEND_NOT_READY,
        message: FRIEND_NOT_READY_TEXT,
      };
    }

    if (!skipChecks) {
      registerPartner(saveData, partnerId);
      if (partnerPayload.gameVersion !== RESERVED_GAME_VERSION) {
        saveData.trainer_house_flag = true;
        saveData.partner_name = partnerPayload.name;
        if (partnerPayload.trainerData) {
          saveData.partner_trainer_data = { ...partnerPayload.trainerData };
        }
      }
    }

    if (partnerPayload.sentDecoration && partnerPayload.decorationId !== undefined && partnerPayload.decorationId !== null) {
      const alreadyOwned = recordDecoration(saveData, partnerPayload.decorationId);
      if (!alreadyOwned) {
        const decorationName = this.resolveDecorationName(partnerPayload);
        const message = this.formatDecorationMessage(
          partnerPayload.name,
          decorationName,
          playerPayload.name
        );
        return {
          status: MysteryGiftOutcomeStatus.DECORATION,
          message,
          partnerName: partnerPayload.name,
          rewardDecoration: partnerPayload.decorationId,
          decorationName,
        };
      }
    }

    const itemIdentifier = partnerPayload.itemName;
    if (!itemIdentifier) {
      throw new MysteryGiftError("Partner payload did not include an item to award.");
    }

    const displayName = this.itemSystem.getDisplayName(itemIdentifier);
    saveData.backup_item = itemIdentifier;
    saveData.stored_item = itemIdentifier;
    return {
      status: MysteryGiftOutcomeStatus.ITEM,
      message: this.formatItemMessage(partnerPayload.name, displayName),
      partnerName: partnerPayload.name,
      rewardItem: itemIdentifier,
      rewardItemDisplay: displayName,
    };
  }

  private resolveDecorationName(payload: MysteryGiftPartnerPayload): string {
    if (payload.decorationName) {
      return payload.decorationName;
    }
    if (payload.decorationId === undefined || payload.decorationId === null) {
      throw new MysteryGiftError("Decoration payload missing identifier.");
    }
    return `Decoration ${payload.decorationId}`.trim();
  }

  private formatItemMessage(partnerName: string, itemName: string): string {
    return `${partnerName} sent\n${itemName}.`;
  }

  private formatDecorationMessage(
    partnerName: string,
    decorationName: string,
    playerName: string
  ): string {
    const owner = playerName || "PLAYER";
    return `${partnerName} sent\n${decorationName}\nto ${owner}'s home.`;
  }
}
