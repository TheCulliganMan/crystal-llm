import path from "path";
import { readJsonAsset, readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import { loadPhoneContactDirectory, primePhoneContactDirectory } from "@pokecrystal/core/ui/menus/pokegear-contacts";
import { YesNoPrompt } from "@pokecrystal/core/ui/text/dialogue";
import { showText as defaultShowText, waitForInput as defaultWaitForInput } from "@pokecrystal/core/engine/events/events";
import type { EventManager } from "@pokecrystal/core/engine/events/events";

export const LOGGER = {
  debug: (...args: unknown[]) => {
    if (isDebugEnabled()) {
      console.debug(...args);
    }
  },
};

export const YesNoBox = YesNoPrompt;

export const MAX_PHONE_CONTACTS = 10;
export const DEFAULT_FRUIT_TREE_ITEM = "BERRY";
const PERMANENT_PHONE_NUMBERS_PATH = path.join(getDataDir(), "permanent_phone_numbers.json");
const INITIALIZE_EVENTS_CONFIG_PATH = path.join(getDataDir(), "initialize_events.json");

export const STANDARD_TEXT_FALLBACKS: Record<string, string> = {
  DifficultBookshelfText: "It's full of\ndifficult books.",
  PictureBookshelfText: "A whole collection\nof #MON picture\nbooks!",
  MagazineBookshelfText: "#MON magazines…\n#MON PAL,\n\n#MON HANDBOOK,\n#MON GRAPH…",
  JackAskNumber1Text:
    "Your knowledge is\nimpressive!\n\nI like that!\n\nWant to trade\nbattle tips?\n\nI'll phone if I\nget good info.\n\nWould you tell me\nyour number?",
  JackAskNumber2Text:
    "Want to trade\nbattle tips?\n\nI'll phone if I\nget good info.\n\nWould you tell me\nyour number?",
  RegisteredNumber1Text: "<PLAYER> registered\n@<STRING_BUFFER_3>'s number.",
  RegisteredNumber2Text: "<PLAYER> registered\n@<STRING_BUFFER_3>'s number.",
  FoundItemText: "<PLAYER> found\n@<STRING_BUFFER_3>!",
  ContestResults_ReadyToJudgeText:
    "We will now judge\nthe #MON you've\ncaught.\n\n<……>\n<……>\n\nWe have chosen the\nwinners!\n\nAre you ready for\nthis?",
  TeamRocketOathText:
    "TEAM ROCKET OATH\n\nSteal #MON for\nprofit!\n\nExploit #MON\nfor profit!\n\nAll #MON exist\nfor the glory of\nTEAM ROCKET!",
  IncenseBurnerText: "What is this?\n\nOh, it's an\nincense burner!",
  MerchandiseShelfText: "Lots of #MON\nmerchandise!",
  LookTownMapText: "It's the TOWN MAP.",
  LookPikachuPosterText: "It's a poster of a cute PIKACHU.",
  LookClefairyPosterText: "It's a poster of a cute CLEFAIRY.",
  LookJigglypuffPosterText: "It's a poster of a cute JIGGLYPUFF.",
  WindowText: "My reflection!\nLookin' good!",
  TVText: "It's a TV.",
  HomepageText: "#MON JOURNAL\nHOME PAGE…\n\nIt hasn't been\nupdated…",
  TrashCanText: "There's nothing in\nhere…",
  PlayersPCTurnOnText: "<PLAYER> turned on\nthe PC.",
  PokecenterPCTurnOnText: "<PLAYER> turned on\nthe PC.",
  PokecenterPCWhoseText: "Access whose PC?",
  PokecenterPCCantUseText: "Bzzzzt! You must\nhave a #MON to\nuse this!",
  PokecenterBillsPCText: "BILL's PC\naccessed.\n\n#MON Storage\nSystem opened.",
  PokecenterPlayersPCText: "Accessed own PC.\n\nItem Storage\nSystem opened.",
  PokecenterOaksPCText: "PROF.OAK's PC\naccessed.\n\n#DEX Rating\nSystem opened.",
  PokecenterPCOaksClosedText: "Turning off the PC.",
  PokecenterSignText: "Heal Your #MON!\n#MON CENTER",
  MartSignText: "For All Your\n#MON Needs\n\n#MON MART",
  CoinVendor_WelcomeText: "Welcome to the\nGAME CORNER.",
  CoinVendor_IntroText: "Do you need some\ngame coins?\n\nIt costs ¥1000 for\n50 coins. Do you\nwant some?",
  CoinVendor_NoCoinCaseText: "Do you need game\ncoins?\n\nOh, you don't have\na COIN CASE for\nyour coins.",
  CoinVendor_Buy50CoinsText: "Thank you!\nHere are 50 coins.",
  CoinVendor_Buy500CoinsText: "Thank you! Here\nare 500 coins.",
  CoinVendor_NotEnoughMoneyText: "You don't have\nenough money.",
  CoinVendor_CoinCaseFullText: "Whoops! Your COIN\nCASE is full.",
  CoinVendor_CancelText: "No coins for you?\nCome again!",
  HappinessText1: "You haven't tamed\nyour #MON.\n\nIf you aren't\nnice, it'll pout.",
  HappinessText2: "#MON get more\nfriendly if you\n\nspend time with\nthem.",
  HappinessText3: "Wow! You and your\n#MON are really\nclose!",
  HappinessNoPokemonText: "You don't have any Pokémon with you.",
  FruitBearingTreeText: "It's a fruit-bearing tree.",
  HeyItsFruitText: "Hey! It's\n@!",
  ObtainedFruitText: "Obtained\n@!",
  FruitPackIsFullText: "But the PACK is full...",
  NothingHereText: "There's nothing here...",
  AskCutText: "This tree can be CUT!\nWant to use CUT?",
  CanCutText: "This tree can be CUT!",
  UseCutText: "@ used\nCUT!",
  UseDigText: "@ used\nDIG!",
  UseEscapeRopeText: "<PLAYER> used an\nESCAPE ROPE.",
  DeleterIntroText:
    "Um... Oh, yes, I'm the MOVE DELETER.\nI can make #MON forget moves.\nShall I make a #MON forget?",
  DeleterAskWhichMonText: "Which #MON?",
  DeleterAskWhichMoveText: "Which move should it forget,\nthen?",
  MoveKnowsOneText: "That #MON knows only one move.",
  AskDeleteMoveText: "Oh, make it forget\n@?",
  DeleterForgotMoveText: "Done! Your #MON forgot the move.",
  MailEggText: "An EGG doesn't know any moves!",
  DeleterNoComeAgainText: "No? Come visit me again.",
  MoveCantForgetHMText: "HM moves can't be forgotten now.",
  UseFishingRodText: "@ used\na FISHING ROD!",
  FishingNothingText: "No fish are biting...",
  CantFishHereText: "There's no water to fish here.",
  CantUseDigText: "Can't use that\nhere.",
  TeleportReturnText: "Return to the last\nPOKEMON CENTER.",
  CantUseTeleportText: "Can't use that\nhere.",
  AskWaterfallText: "Do you want to use\nWATERFALL?",
  HugeWaterfallText: "Wow, it's a huge\nwaterfall.",
  UseWaterfallText: "@ used\nWATERFALL!",
  BlindingFlashText: "A blinding FLASH lights the area!",
  UseFlashTextScript: "A blinding FLASH lights the area!",
  UseRockSmashText: "@ used\nROCK SMASH!",
  AskRockSmashText: "Use ROCK SMASH?",
  MaySmashText: "Maybe a #MON\ncan break this.",
  AskStrengthText: "A #MON may be\nable to move this.\n\nWant to use\nSTRENGTH?",
  UseStrengthText: "@ used\nSTRENGTH!",
  CantUseItemText: "Can't use that here.",
  CantSurfText: "Can't use SURF here.",
  UsedSurfText: "@ used\nSURF!",
  AlreadySurfingText: "You're already\nSURFING.",
  AskSurfText: "The water is calm.\nWant to SURF?",
  CutNothingText: "There's nothing to CUT here.",
  UseSweetScentText: "@ used\nSWEET SCENT!",
  SweetScentNothingText: "Looks like there's\nnothing here...",
  RepelUseText: "REPEL will keep weak #MON away.",
  RepelUsedEarlierIsStillInEffectText: "The REPEL used earlier is still in effect.",
  RepelWoreOffText: "REPEL's effect wore off.",
  RodBiteText: "Oh! A bite!",
  RodNothingText: "Not even a nibble...",
  EvolvingText: "What? @ is evolving!",
  EvolvedIntoText: "Congratulations!\n@ evolved into @!",
  StoppedEvolvingText: "Huh? @ stopped evolving!",
  GymStatue_CityGymText: "@\n+#MON GYM",
  GymStatue_WinningTrainersText: "LEADER: @\n+WINNING TRAINERS:\n+<PLAYER>",
};

export const PHONE_CONTACT_BASE_NAMES_MALE: Record<string, string> = {
  PHONE_SCHOOLBOY_JACK: "Jack",
  PHONE_SAILOR_HUEY: "Huey",
  PHONE_COOLTRAINERM_GAVEN: "Gaven",
  PHONE_BIRDKEEPER_JOSE: "Jose",
  PHONE_YOUNGSTER_JOEY: "Joey",
  PHONE_BUG_CATCHER_WADE: "Wade",
  PHONE_FISHER_RALPH: "Ralph",
  PHONE_HIKER_ANTHONY: "Anthony",
  PHONE_CAMPER_TODD: "Todd",
  PHONE_JUGGLER_IRWIN: "Irwin",
  PHONE_BUG_CATCHER_ARNIE: "Arnie",
  PHONE_SCHOOLBOY_ALAN: "Alan",
  PHONE_SCHOOLBOY_CHAD: "Chad",
  PHONE_POKEFANM_DEREK: "Derek",
  PHONE_FISHER_TULLY: "Tully",
  PHONE_POKEMANIAC_BRENT: "Brent",
  PHONE_BIRDKEEPER_VANCE: "Vance",
  PHONE_FISHER_WILTON: "Wilton",
  PHONE_BLACKBELT_KENJI: "Kenji",
  PHONE_HIKER_PARRY: "Parry",
};

export const PHONE_CONTACT_BASE_NAMES_FEMALE: Record<string, string> = {
  PHONE_POKEFAN_BEVERLY: "Beverly",
  PHONE_COOLTRAINERF_BETH: "Beth",
  PHONE_COOLTRAINERF_REENA: "Reena",
  PHONE_PICNICKER_LIZ: "Liz",
  PHONE_PICNICKER_GINA: "Gina",
  PHONE_LASS_DANA: "Dana",
  PHONE_PICNICKER_TIFFANY: "Tiffany",
  PHONE_PICNICKER_ERIN: "Erin",
};

export const normalizePhoneNumber = (token: string | null | undefined): string => {
  if (!token) {
    return "";
  }
  return token.trim().replace(/,$/, "");
};

export const resolvePhoneContactId = (token: string | null | undefined): string => {
  const normalized = normalizePhoneNumber(token);
  if (!normalized) {
    return "";
  }
  const directory = loadPhoneContactDirectory();
  return resolvePhoneContactToken(normalized, directory) ?? normalized;
};

type StoryEventRuntimeCache = {
  permanentNumbers: string[] | null;
  initializeConfig: InitializeEventsConfig | null;
};

const getStoryEventRuntimeCache = (): StoryEventRuntimeCache => {
  const scope = globalThis as typeof globalThis & {
    __POKECRYSTAL_STORY_EVENT_RUNTIME_CACHE__?: StoryEventRuntimeCache;
  };
  if (!scope.__POKECRYSTAL_STORY_EVENT_RUNTIME_CACHE__) {
    scope.__POKECRYSTAL_STORY_EVENT_RUNTIME_CACHE__ = {
      permanentNumbers: null,
      initializeConfig: null,
    };
  }
  return scope.__POKECRYSTAL_STORY_EVENT_RUNTIME_CACHE__;
};

const readRequiredStoryEventAsset = <T>(assetPath: string, description: string): T => {
  try {
    return readJsonAssetSync<T>(assetPath);
  } catch {
    throw new Error(
      `${description} is required for the asset-only runtime: missing or invalid ${assetPath}.`
    );
  }
};

const readRequiredStoryEventAssetAsync = async <T>(
  assetPath: string,
  description: string,
): Promise<T> => {
  try {
    return await readJsonAsset<T>(assetPath);
  } catch {
    throw new Error(
      `${description} is required for the asset-only runtime: missing or invalid ${assetPath}.`
    );
  }
};

export const loadPermanentPhoneNumbers = (): string[] => {
  const cache = getStoryEventRuntimeCache();
  if (cache.permanentNumbers) {
    return cache.permanentNumbers;
  }
  const bundled = readRequiredStoryEventAsset<unknown[]>(
    PERMANENT_PHONE_NUMBERS_PATH,
    "Permanent phone numbers"
  );
  if (!Array.isArray(bundled)) {
    throw new Error(
      `Permanent phone numbers is required for the asset-only runtime: missing or invalid ${PERMANENT_PHONE_NUMBERS_PATH}.`
    );
  }
  const directory = loadPhoneContactDirectory();
  const numbers: string[] = [];
  for (const token of bundled) {
    if (typeof token !== "string") {
      continue;
    }
    const resolved = resolvePhoneContactToken(token, directory);
    if (resolved) {
      numbers.push(resolved);
    }
  }
  cache.permanentNumbers = Array.from(new Set(numbers));
  return cache.permanentNumbers;
};

export const tryAddPhoneNumber = (phoneNumbers: string[], token: string): boolean => {
  const directory = loadPhoneContactDirectory();
  const normalized = resolvePhoneContactToken(token, directory);
  if (!normalized || normalized === "PHONE_00") {
    return false;
  }
  const normalizedList = phoneNumbers.map(
    (entry) => resolvePhoneContactToken(entry, directory) ?? normalizePhoneNumber(entry),
  );
  if (normalizedList.includes(normalized)) {
    return false;
  }
  const missing = countMissingPermanentNumbers(normalizedList, normalized);
  const maxContacts = MAX_PHONE_CONTACTS - missing;
  if (normalizedList.length >= maxContacts) {
    return false;
  }
  phoneNumbers.push(normalized);
  return true;
};

const countMissingPermanentNumbers = (phoneNumbers: string[], candidate: string): number => {
  let missing = 0;
  for (const permanent of loadPermanentPhoneNumbers()) {
    if (permanent === candidate) {
      continue;
    }
    if (!phoneNumbers.includes(permanent)) {
      missing += 1;
    }
  }
  return missing;
};

const resolvePhoneContactToken = (
  token: string,
  directory: ReturnType<typeof loadPhoneContactDirectory>,
): string | null => {
  const normalized = normalizePhoneNumber(token);
  if (!normalized) {
    return null;
  }
  const resolved = directory.resolveContactId(normalized);
  if (resolved) {
    return resolved;
  }
  return normalized;
};

export const DAY_NAME_LABELS = [
  "SUNDAY",
  "MONDAY",
  "TUESDAY",
  "WEDNESDAY",
  "THURSDAY",
  "FRIDAY",
  "SATURDAY",
] as const;

export type InitializeEventsConfig = {
  eventFlags: string[];
  engineFlags: string[];
  variableSprites: Record<string, string>;
};

export const loadInitializeEventsConfig = (): InitializeEventsConfig => {
  const cache = getStoryEventRuntimeCache();
  if (cache.initializeConfig) {
    return cache.initializeConfig;
  }
  const bundled = readRequiredStoryEventAsset<Partial<InitializeEventsConfig>>(
    INITIALIZE_EVENTS_CONFIG_PATH,
    "Initialize events config"
  );
  const eventFlags = Array.isArray(bundled?.eventFlags)
    ? bundled.eventFlags.filter((flag): flag is string => typeof flag === "string")
    : [];
  const engineFlags = Array.isArray(bundled?.engineFlags)
    ? bundled.engineFlags.filter((flag): flag is string => typeof flag === "string")
    : [];
  const variableSprites: Record<string, string> = {};
  if (bundled?.variableSprites && typeof bundled.variableSprites === "object") {
    for (const [spriteId, replacement] of Object.entries(bundled.variableSprites)) {
      if (typeof replacement === "string") {
        variableSprites[spriteId] = replacement;
      }
    }
  }
  if (!eventFlags.length && !engineFlags.length && !Object.keys(variableSprites).length) {
    throw new Error(
      `Initialize events config is required for the asset-only runtime: missing or invalid ${INITIALIZE_EVENTS_CONFIG_PATH}.`
    );
  }

  const uniqueEvents = Array.from(new Set(eventFlags));
  const uniqueEngineFlags = Array.from(new Set(engineFlags));
  cache.initializeConfig = {
    eventFlags: uniqueEvents,
    engineFlags: uniqueEngineFlags,
    variableSprites,
  };
  return cache.initializeConfig;
};

export const primeStoryEventRuntimeAssets = async (): Promise<void> => {
  const cache = getStoryEventRuntimeCache();
  if (cache.permanentNumbers && cache.initializeConfig) {
    return;
  }

  const [directory, bundledPermanentNumbers, bundledInitializeConfig] = await Promise.all([
    primePhoneContactDirectory(),
    cache.permanentNumbers
      ? Promise.resolve<unknown[]>(cache.permanentNumbers)
      : readRequiredStoryEventAssetAsync<unknown[]>(
          PERMANENT_PHONE_NUMBERS_PATH,
          "Permanent phone numbers"
        ),
    cache.initializeConfig
      ? Promise.resolve<Partial<InitializeEventsConfig>>(cache.initializeConfig)
      : readRequiredStoryEventAssetAsync<Partial<InitializeEventsConfig>>(
          INITIALIZE_EVENTS_CONFIG_PATH,
          "Initialize events config"
        ),
  ]);

  if (!cache.permanentNumbers) {
    if (!Array.isArray(bundledPermanentNumbers)) {
      throw new Error(
        `Permanent phone numbers is required for the asset-only runtime: missing or invalid ${PERMANENT_PHONE_NUMBERS_PATH}.`
      );
    }
    const numbers: string[] = [];
    for (const token of bundledPermanentNumbers) {
      if (typeof token !== "string") {
        continue;
      }
      const resolved = resolvePhoneContactToken(token, directory);
      if (resolved) {
        numbers.push(resolved);
      }
    }
    cache.permanentNumbers = Array.from(new Set(numbers));
  }

  if (!cache.initializeConfig) {
    const eventFlags = Array.isArray(bundledInitializeConfig?.eventFlags)
      ? bundledInitializeConfig.eventFlags.filter((flag): flag is string => typeof flag === "string")
      : [];
    const engineFlags = Array.isArray(bundledInitializeConfig?.engineFlags)
      ? bundledInitializeConfig.engineFlags.filter((flag): flag is string => typeof flag === "string")
      : [];
    const variableSprites: Record<string, string> = {};
    if (bundledInitializeConfig?.variableSprites && typeof bundledInitializeConfig.variableSprites === "object") {
      for (const [spriteId, replacement] of Object.entries(bundledInitializeConfig.variableSprites)) {
        if (typeof replacement === "string") {
          variableSprites[spriteId] = replacement;
        }
      }
    }
    if (!eventFlags.length && !engineFlags.length && !Object.keys(variableSprites).length) {
      throw new Error(
        `Initialize events config is required for the asset-only runtime: missing or invalid ${INITIALIZE_EVENTS_CONFIG_PATH}.`
      );
    }
    cache.initializeConfig = {
      eventFlags: Array.from(new Set(eventFlags)),
      engineFlags: Array.from(new Set(engineFlags)),
      variableSprites,
    };
  }
};

export const showText = (eventManager: EventManager, text: string, data: Record<string, unknown> = {}): void => {
  defaultShowText(eventManager, text, data);
};

export const waitForInput = (eventManager: EventManager, pauseRunner = false): void => {
  defaultWaitForInput(eventManager, { pauseRunner });
};

const isDebugEnabled = (): boolean => {
  const level = process.env.POKECRYSTAL_LOG_LEVEL?.toLowerCase();
  return level === "debug";
};
