import path from "path";
import { readJsonAsset, readJsonAssetSync } from "../../core/asset-reader";
import { getDataDir } from "../../core/paths";

export type PhoneContactRecord = {
  contactId: string;
  trainerClass: string | null;
  trainerLabel: string | null;
  lines: [string, ...string[]];
  primaryLabel: string;
  mapConstant: string | null;
  calleeTimeMask: number;
  calleeScript: string | null;
  callerTimeMask: number;
  callerScript: string | null;
};

const NON_TRAINER_CONTACT_IDS: Record<string, string> = {
  PHONECONTACT_MOM: "PHONE_MOM",
  PHONECONTACT_BIKESHOP: "PHONE_OAK",
  PHONECONTACT_BILL: "PHONE_BILL",
  PHONECONTACT_ELM: "PHONE_ELM",
  PHONECONTACT_BUENA: "PHONE_BUENA",
};

const CONTACT_ID_ALIASES: Record<string, string> = {
  ...NON_TRAINER_CONTACT_IDS,
  PHONE_BIKESHOP: "PHONE_OAK",
  PHONE_BIKE_SHOP: "PHONE_OAK",
};
const PHONE_CONTACTS_JSON_PATH = path.join(getDataDir(), "phone_contacts.json");

type PhoneContactRuntimeCache = {
  directory: PhoneContactDirectory | null;
};

const getPhoneContactRuntimeCache = (): PhoneContactRuntimeCache => {
  const scope = globalThis as typeof globalThis & {
    __POKECRYSTAL_PHONE_CONTACT_RUNTIME_CACHE__?: PhoneContactRuntimeCache;
  };
  if (!scope.__POKECRYSTAL_PHONE_CONTACT_RUNTIME_CACHE__) {
    scope.__POKECRYSTAL_PHONE_CONTACT_RUNTIME_CACHE__ = {
      directory: null,
    };
  }
  return scope.__POKECRYSTAL_PHONE_CONTACT_RUNTIME_CACHE__;
};

export class PhoneContactDirectory {
  private readonly records: Record<string, PhoneContactRecord>;
  private readonly trainerLabelToContactId: Record<string, string>;

  constructor(records?: Record<string, PhoneContactRecord>) {
    const bundledRecords = records ?? loadBundledPhoneContacts();
    this.records = bundledRecords;
    this.trainerLabelToContactId = buildTrainerLabelIndex(bundledRecords);
  }

  displayLines(contactId: string): [string, ...string[]] {
    const record = this.record(contactId);
    if (record) {
      return record.lines;
    }
    throw new Error(`Unknown phone contact '${contactId}'.`);
  }

  primaryLabel(contactId: string): string {
    const record = this.record(contactId);
    if (record) {
      return record.primaryLabel;
    }
    throw new Error(`Unknown phone contact '${contactId}'.`);
  }

  record(contactId: string): PhoneContactRecord | null {
    return this.records[normalizeContactId(contactId)] ?? null;
  }

  resolveContactId(token: string): string | null {
    const normalized = normalizeContactId(token);
    if (!normalized) {
      return null;
    }
    if (normalized.startsWith("PHONE_")) {
      return this.records[normalized] ? normalized : null;
    }
    const contactId = this.contactIdForTrainerLabel(normalized);
    if (contactId) {
      return contactId;
    }
    if (normalized.startsWith("PHONECONTACT_")) {
      return normalizeContactId(`PHONE_${normalized.slice("PHONECONTACT_".length)}`);
    }
    return normalized;
  }

  contactIdForTrainerLabel(trainerLabel: string): string | null {
    const normalized = trainerLabel.trim().replace(/,$/, "");
    if (!normalized) {
      return null;
    }
    return this.trainerLabelToContactId[normalized] ?? null;
  }
}

function normalizeContactId(contactId: string): string {
  const normalized = contactId.trim().replace(/,$/, "");
  return CONTACT_ID_ALIASES[normalized] ?? normalized;
}

function buildTrainerLabelIndex(records: Record<string, PhoneContactRecord>): Record<string, string> {
  const trainerLabelToContactId: Record<string, string> = {};
  for (const record of Object.values(records)) {
    if (record.trainerLabel) {
      trainerLabelToContactId[record.trainerLabel] = record.contactId;
    }
  }
  return trainerLabelToContactId;
}

function loadBundledPhoneContacts(): Record<string, PhoneContactRecord> {
  try {
    return parseBundledPhoneContacts(
      readJsonAssetSync<Record<string, PhoneContactRecord>>(PHONE_CONTACTS_JSON_PATH)
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Phone contact directory requires bundled asset ${PHONE_CONTACTS_JSON_PATH}: ${message}`,
    );
  }
}

function parseBundledPhoneContacts(
  raw: Record<string, PhoneContactRecord>
): Record<string, PhoneContactRecord> {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error(
      `Phone contact directory requires bundled asset ${PHONE_CONTACTS_JSON_PATH} to contain a contact map.`,
    );
  }
  const records: Record<string, PhoneContactRecord> = {};
  for (const [contactId, entry] of Object.entries(raw)) {
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
      continue;
    }
    const lines = Array.isArray(entry.lines) ? entry.lines.filter((line): line is string => typeof line === "string") : [];
    if (!lines.length) {
      continue;
    }
    records[contactId] = {
      contactId: String(entry.contactId ?? contactId),
      trainerClass: entry.trainerClass ?? null,
      trainerLabel: entry.trainerLabel ?? null,
      lines: [lines[0], ...lines.slice(1)],
      primaryLabel: String(entry.primaryLabel ?? "").trim() || String(contactId).replace(/_/g, " "),
      mapConstant: entry.mapConstant ?? null,
      calleeTimeMask: Number(entry.calleeTimeMask ?? 0) || 0,
      calleeScript: entry.calleeScript ?? null,
      callerTimeMask: Number(entry.callerTimeMask ?? 0) || 0,
      callerScript: entry.callerScript ?? null,
    };
  }
  if (!Object.keys(records).length) {
    throw new Error(
      `Phone contact directory requires bundled asset ${PHONE_CONTACTS_JSON_PATH} to include at least one valid record.`,
    );
  }
  return records;
}

export function loadPhoneContactDirectory(): PhoneContactDirectory {
  const cache = getPhoneContactRuntimeCache();
  if (!cache.directory) {
    cache.directory = new PhoneContactDirectory();
  }
  return cache.directory;
}

export async function primePhoneContactDirectory(): Promise<PhoneContactDirectory> {
  const cache = getPhoneContactRuntimeCache();
  if (cache.directory) {
    return cache.directory;
  }
  try {
    const raw = await readJsonAsset<Record<string, PhoneContactRecord>>(PHONE_CONTACTS_JSON_PATH);
    cache.directory = new PhoneContactDirectory(parseBundledPhoneContacts(raw));
    return cache.directory;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Phone contact directory requires bundled asset ${PHONE_CONTACTS_JSON_PATH}: ${message}`,
    );
  }
}
