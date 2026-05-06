import { Command } from './commands/base';
import { WriteTextCommand, OpenTextCommand, CloseTextCommand, WaitButtonCommand, YesOrNoCommand, GetStringCommand, TradeCommand } from './commands/text';
import {
    AppearCommand,
    ApplyMovementLastTalkedCommand,
    ApplyMovementCommand,
    BattleTowerTextCommand,
    ChangeBlockCommand,
    CheckPokeMailCommand,
    GivePokeMailCommand,
    CheckTimeCommand,
    CloseWindowCommand,
    ClosePokePicCommand,
    CryCommand,
    DisappearCommand,
    EarthquakeCommand,
    FaceObjectCommand,
    FacePlayerCommand,
    FollowCommand,
    GetMonNameCommand,
    LoadMenuCommand,
    LockAllCommand,
    LockCommand,
    MoveObjectCommand,
    PauseCommand,
    PlaySoundCommand,
    PokePicCommand,
    RefreshMapCommand,
    ReleaseAllCommand,
    ReleaseCommand,
    ShowEmoteCommand,
    StopCommand,
    StopFollowCommand,
    SwarmCommand,
    TurnObjectCommand,
    VerticalMenuCommand,
    WaitSFXCommand,
    WarpCheckCommand,
    WarpCommand,
    WarpFacingCommand,
    WarpSoundCommand,
    NewLoadMapCommand,
    ElevatorCommand,
} from './commands/overworld';
import {
    DontRestartMapMusicCommand,
    JumpCommand,
    MusicFadeOutCommand,
    PlayMusicCommand,
    ReloadMapAfterBattleCommand,
    ScriptCallCommand,
    SdeferCommand,
} from './commands/movement';
import { SpecialCommand } from './commands/special';
import { CreditsCommand } from './commands/credits';
import {
    BattleCommand,
    CatchTutorialCommand,
    EndCommand,
    GiveEggCommand,
    GivePokeCommand,
    LoadTrainerCommand,
    LoadWildMonCommand,
    StartBattleCommand,
    TrainerCommand,
    WinLossTextCommand,
} from './commands/battle';
import {
    BlackoutModCommand,
    CheckEventCommand,
    CheckFlagCommand,
    CheckSceneCommand,
    CheckVersionCommand,
    ClearEventCommand,
    ClearFlagCommand,
    CallAsmCommand,
    ConditionalEventCommand,
    DescribeDecorationCommand,
    EndIfJustBattledCommand,
    IfEqualCommand,
    IfFalseCommand,
    IfGreaterCommand,
    IfLessCommand,
    IfNotEqualCommand,
    IfTrueCommand,
    JumpStandardCommand,
    SetEngineFlagCommand,
    SetEventCommand,
    SetFlagCommand,
    SetLastTalkedCommand,
    SetMapSceneCommand,
    SetSceneCommand,
    SpecialPhoneCallCommand,
    VariableSpriteCommand,
    RandomCommand,
} from './commands/events';
import { FruitTreeCommand } from './commands/fruit-tree';
import { HallOfFameCommand } from './commands/hall-of-fame';
import {
    AddCellNumCommand,
    AskForPhoneNumberCommand,
    CheckCellNumCommand,
    CheckCoinsCommand,
    CheckMoneyCommand,
    CheckPokeCommand,
    CheckItemCommand,
    GetNumCommand,
    GetItemNameCommand,
    GetTrainerNameCommand,
    GiveCoinsCommand,
    GiveItemCommand,
    HiddenItemCommand,
    ItemBallCommand,
    ItemNotifyCommand,
    LoadVarCommand,
    PlayMapMusicCommand,
    PokemartCommand,
    ReadVarCommand,
    ReanchorMapCommand,
    ReloadMapCommand,
    SetValCommand,
    TakeCoinsCommand,
    TakeMoneyCommand,
    TakeItemCommand,
    VerboseGiveItemVarCommand,
    VerboseGiveItemCommand,
    WriteVarCommand,
} from './commands/items';
import { AddValCommand, LoadMemCommand, ReadMemCommand, WriteMemCommand } from './commands/memory';
import { ScriptRunner } from './runner';

type CommandCreator = (...args: string[]) => Command;

const parseScriptInt = (token: string | undefined): number => {
    if (typeof token !== "string") {
        throw new Error("Cannot parse an undefined numeric token.");
    }
    // Strip ASM-style comments (e.g., "$2e ; closed wall" -> "$2e")
    const commentIndex = token.indexOf(";");
    const withoutComment = commentIndex !== -1 ? token.slice(0, commentIndex) : token;
    const normalized = withoutComment.trim();
    if (!normalized) {
        throw new Error("Cannot parse an empty numeric token.");
    }
    let sign = 1;
    let valueToken = normalized;
    if (valueToken.startsWith("-")) {
        sign = -1;
        valueToken = valueToken.slice(1);
    } else if (valueToken.startsWith("+")) {
        valueToken = valueToken.slice(1);
    }
    let base = 10;
    if (valueToken.startsWith("$")) {
        base = 16;
        valueToken = valueToken.slice(1);
    } else if (valueToken.toLowerCase().startsWith("0x")) {
        base = 16;
        valueToken = valueToken.slice(2);
    }
    if (!valueToken) {
        throw new Error(`Numeric token '${token}' does not contain digits.`);
    }
    const parsed = parseInt(valueToken, base);
    if (Number.isNaN(parsed)) {
        throw new Error(`Numeric token '${token}' could not be parsed.`);
    }
    return sign * parsed;
};

const decodeEarthquakeParam = (token: string): { intensity: number; duration: number } => {
    const encoded = parseScriptInt(token);
    if (!Number.isInteger(encoded) || encoded < 0 || encoded > 0xff) {
        throw new Error(`earthquake expects a single byte parameter, got '${token}'.`);
    }
    // ASM mapping:
    // - engine/overworld/scripting.asm::Script_earthquake (single byte param)
    // - engine/overworld/map_objects.asm::MovementFunction_ScreenShake.GetDurationAndField1e
    const duration = encoded & 0b0011_1111;
    const intensity = 1 << ((encoded >> 6) & 0b0000_0011);
    return { intensity, duration };
};

export class CommandFactory {
    private runner: ScriptRunner;
    public commandMap: Map<string, CommandCreator>;

    constructor(runner: ScriptRunner) {
        this.runner = runner;
        this.commandMap = new Map<string, CommandCreator>([
            ['writetext', this.createWriteTextCommand.bind(this)],
            ['farwritetext', this.createWriteTextCommand.bind(this)], // Alias
            ['opentext', this.createOpenTextCommand.bind(this)],
            ['closetext', this.createCloseTextCommand.bind(this)],
            ['closewindow', this.createCloseWindowCommand.bind(this)],
            ['battletowertext', this.createBattleTowerTextCommand.bind(this)],
            ['waitbutton', this.createWaitButtonCommand.bind(this)],
            ['promptbutton', this.createWaitButtonCommand.bind(this)], // Alias
            ['yesorno', this.createYesOrNoCommand.bind(this)],
            ['loadmenu', this.createLoadMenuCommand.bind(this)],
            ['verticalmenu', this.createVerticalMenuCommand.bind(this)],
            ['getstring', this.createGetStringCommand.bind(this)],
            ['trade', this.createTradeCommand.bind(this)],
            ['credits', this.createCreditsCommand.bind(this)],
            ['sjump', this.createJumpCommand.bind(this)],
            ['jump', this.createJumpCommand.bind(this)], // Alias
            ['sdefer', this.createSdeferCommand.bind(this)],
            ['playmusic', this.createPlayMusicCommand.bind(this)],
            ['musicfadeout', this.createMusicFadeOutCommand.bind(this)],
            ['dontrestartmapmusic', this.createDontRestartMapMusicCommand.bind(this)],
            ['reloadmapafterbattle', this.createReloadMapAfterBattleCommand.bind(this)],
            ['battle', this.createBattleCommand.bind(this)],
            ['trainer', this.createTrainerCommand.bind(this)],
            ['loadtrainer', this.createLoadTrainerCommand.bind(this)],
            ['startbattle', this.createStartBattleCommand.bind(this)],
            ['winlosstext', this.createWinLossTextCommand.bind(this)],
            ['givepoke', this.createGivePokeCommand.bind(this)],
            ['giveegg', this.createGiveEggCommand.bind(this)],
            ['loadwildmon', this.createLoadWildMonCommand.bind(this)],
            ['catchtutorial', this.createCatchTutorialCommand.bind(this)],
            ['special', this.createSpecialCommand.bind(this)],
            ['callasm', this.createCallAsmCommand.bind(this)],
            ['follow', this.createFollowCommand.bind(this)],
            ['stopfollow', this.createStopFollowCommand.bind(this)],
            ['lock', this.createLockCommand.bind(this)],
            ['release', this.createReleaseCommand.bind(this)],
            ['lockall', this.createLockAllCommand.bind(this)],
            ['releaseall', this.createReleaseAllCommand.bind(this)],
            ['stop', this.createStopCommand.bind(this)],
            ['playsound', this.createPlaySoundCommand.bind(this)],
            ['waitsfx', this.createWaitSFXCommand.bind(this)],
            ['pause', this.createPauseCommand.bind(this)],
            ['earthquake', this.createEarthquakeCommand.bind(this)],
            ['cry', this.createCryCommand.bind(this)],
            ['pokepic', this.createPokePicCommand.bind(this)],
            ['closepokepic', this.createClosePokePicCommand.bind(this)],
            ['getmonname', this.createGetMonNameCommand.bind(this)],
            ['appear', this.createAppearCommand.bind(this)],
            ['disappear', this.createDisappearCommand.bind(this)],
            ['checkpokemail', this.createCheckPokeMailCommand.bind(this)],
            ['givepokemail', this.createGivePokeMailCommand.bind(this)],
            ['checktime', this.createCheckTimeCommand.bind(this)],
            ['moveobject', this.createMoveObjectCommand.bind(this)],
            ['changeblock', this.createChangeBlockCommand.bind(this)],
            ['refreshmap', this.createRefreshMapCommand.bind(this)],
            ['warp', this.createWarpCommand.bind(this)],
            ['warpfacing', this.createWarpFacingCommand.bind(this)],
            ['warpcheck', this.createWarpCheckCommand.bind(this)],
            ['warpsound', this.createWarpSoundCommand.bind(this)],
            ['newloadmap', this.createNewLoadMapCommand.bind(this)],
            ['swarm', this.createSwarmCommand.bind(this)],
            ['elevator', this.createElevatorCommand.bind(this)],
            ['turnobject', this.createTurnObjectCommand.bind(this)],
            ['faceobject', this.createFaceObjectCommand.bind(this)],
            ['showemote', this.createShowEmoteCommand.bind(this)],
            ['applymovement', this.createApplyMovementCommand.bind(this)],
            ['applymovementlasttalked', this.createApplyMovementLastTalkedCommand.bind(this)],
            ['faceplayer', this.createFacePlayerCommand.bind(this)],
            ['scall', this.createScriptCallCommand.bind(this)],
            ['setmapscene', this.createSetMapSceneCommand.bind(this)],
            ['checkscene', this.createCheckSceneCommand.bind(this)],
            ['clearflag', this.createClearFlagCommand.bind(this)],
            ['clear_flag', this.createClearFlagCommand.bind(this)],
            ['setflag', this.createSetFlagCommand.bind(this)],
            ['set_flag', this.createSetFlagCommand.bind(this)],
            ['setengineflag', this.createSetEngineFlagCommand.bind(this)],
            ['variablesprite', this.createVariableSpriteCommand.bind(this)],
            ['describedecoration', this.createDescribeDecorationCommand.bind(this)],
            ['checkflag', this.createCheckFlagCommand.bind(this)],
            ['check_flag', this.createCheckFlagCommand.bind(this)],
            ['conditional_event', this.createConditionalEventCommand.bind(this)],
            ['setevent', this.createSetEventCommand.bind(this)],
            ['checkevent', this.createCheckEventCommand.bind(this)],
            ['specialphonecall', this.createSpecialPhoneCallCommand.bind(this)],
            ['iftrue', this.createIfTrueCommand.bind(this)],
            ['iffalse', this.createIfFalseCommand.bind(this)],
            ['setscene', this.createSetSceneCommand.bind(this)],
            ['clearevent', this.createClearEventCommand.bind(this)],
            ['blackoutmod', this.createBlackoutModCommand.bind(this)],
            ['setlasttalked', this.createSetLastTalkedCommand.bind(this)],
            ['endifjustbattled', this.createEndIfJustBattledCommand.bind(this)],
            ['end', this.createEndCommand.bind(this)],
            ['endcallback', this.createEndCommand.bind(this)],
            ['ifequal', this.createIfEqualCommand.bind(this)],
            ['ifnotequal', this.createIfNotEqualCommand.bind(this)],
            ['ifgreater', this.createIfGreaterCommand.bind(this)],
            ['ifless', this.createIfLessCommand.bind(this)],
            ['jumpstd', this.createJumpStandardCommand.bind(this)],
            ['verbosegiveitem', this.createVerboseGiveItemCommand.bind(this)],
            ['verbosegiveitemvar', this.createVerboseGiveItemVarCommand.bind(this)],
            ['askforphonenumber', this.createAskForPhoneNumberCommand.bind(this)],
            ['checkcellnum', this.createCheckCellNumCommand.bind(this)],
            ['checkitem', this.createCheckItemCommand.bind(this)],
            ['checkcoins', this.createCheckCoinsCommand.bind(this)],
            ['checkmoney', this.createCheckMoneyCommand.bind(this)],
            ['checkpoke', this.createCheckPokeCommand.bind(this)],
            ['checkver', this.createCheckVerCommand.bind(this)],
            ['givecoins', this.createGiveCoinsCommand.bind(this)],
            ['takecoins', this.createTakeCoinsCommand.bind(this)],
            ['takemoney', this.createTakeMoneyCommand.bind(this)],
            ['itemball', this.createItemBallCommand.bind(this)],
            ['hiddenitem', this.createHiddenItemCommand.bind(this)],
            ['itemnotify', this.createItemNotifyCommand.bind(this)],
            ['loadvar', this.createLoadVarCommand.bind(this)],
            ['readvar', this.createReadVarCommand.bind(this)],
            ['writevar', this.createWriteVarCommand.bind(this)],
            ['setval', this.createSetValCommand.bind(this)],
            ['random', this.createRandomCommand.bind(this)],
            ['getnum', this.createGetNumCommand.bind(this)],
            ['playmapmusic', this.createPlayMapMusicCommand.bind(this)],
            ['reloadmap', this.createReloadMapCommand.bind(this)],
            ['reanchormap', this.createReanchorMapCommand.bind(this)],
            ['takeitem', this.createTakeItemCommand.bind(this)],
            ['getitemname', this.createGetItemNameCommand.bind(this)],
            ['gettrainername', this.createGetTrainerNameCommand.bind(this)],
            ['giveitem', this.createGiveItemCommand.bind(this)],
            ['pokemart', this.createPokemartCommand.bind(this)],
            ['addcellnum', this.createAddCellNumCommand.bind(this)],
            ['readmem', this.createReadMemCommand.bind(this)],
            ['writemem', this.createWriteMemCommand.bind(this)],
            ['loadmem', this.createLoadMemCommand.bind(this)],
            ['addval', this.createAddValCommand.bind(this)],
            ['fruittree', this.createFruitTreeCommand.bind(this)],
            ['halloffame', this.createHallOfFameCommand.bind(this)],
        ]);
    }

    private createWriteTextCommand(textLabel: string): Command {
        return new WriteTextCommand(textLabel);
    }

    private createOpenTextCommand(): Command {
        return new OpenTextCommand();
    }

    private createCloseTextCommand(): Command {
        return new CloseTextCommand();
    }

    private createCloseWindowCommand(): Command {
        return new CloseWindowCommand();
    }

    private createBattleTowerTextCommand(textLabel: string): Command {
        return new BattleTowerTextCommand(textLabel);
    }

    private createWaitButtonCommand(): Command {
        return new WaitButtonCommand();
    }

    private createYesOrNoCommand(): Command {
        return new YesOrNoCommand();
    }

    private createLoadMenuCommand(menuHeaderLabel: string): Command {
        return new LoadMenuCommand(menuHeaderLabel);
    }

    private createVerticalMenuCommand(): Command {
        return new VerticalMenuCommand();
    }

    private createGetStringCommand(bufferName: string, textLabel: string): Command {
        return new GetStringCommand(bufferName, textLabel);
    }

    private createTradeCommand(tradeId: string): Command {
        return new TradeCommand(tradeId);
    }

    private createCreditsCommand(allowSkip?: string): Command {
        if (allowSkip === undefined) {
            return new CreditsCommand();
        }
        const normalized = allowSkip.trim().toLowerCase();
        if (!normalized) {
            return new CreditsCommand();
        }
        const parsed = normalized === "true" || normalized === "1";
        return new CreditsCommand(parsed);
    }

    private createJumpCommand(scriptName: string): Command {
        return new JumpCommand(scriptName);
    }

    private createSdeferCommand(scriptName: string): Command {
        return new SdeferCommand(scriptName);
    }

    private createScriptCallCommand(scriptName: string): Command {
        return new ScriptCallCommand(scriptName);
    }

    private createPlayMusicCommand(musicId: string): Command {
        return new PlayMusicCommand(musicId);
    }

    private createMusicFadeOutCommand(musicId: string, speedFrames: string): Command {
        return new MusicFadeOutCommand(musicId, parseScriptInt(speedFrames));
    }

    private createDontRestartMapMusicCommand(): Command {
        return new DontRestartMapMusicCommand();
    }

    private createReloadMapAfterBattleCommand(): Command {
        return new ReloadMapAfterBattleCommand();
    }

    private createBattleCommand(trainerName: string): Command {
        return new BattleCommand(trainerName);
    }

    private createTrainerCommand(...args: string[]): Command {
        if (args.length < 2) {
            throw new Error("TrainerCommand requires at least a class and trainer id.");
        }
        const padded = args.slice(0, 7);
        while (padded.length < 7) {
            padded.push("0");
        }
        const [
            trainerClass,
            trainerId,
            eventFlag,
            seenText,
            winText,
            lossText,
            callback,
        ] = padded;
        return new TrainerCommand(trainerClass, trainerId, eventFlag, seenText, winText, lossText, callback);
    }

    private createLoadTrainerCommand(trainerClass: string, trainerId: string): Command {
        return new LoadTrainerCommand(trainerClass, trainerId);
    }

    private createStartBattleCommand(): Command {
        return new StartBattleCommand();
    }

    private createWinLossTextCommand(winTextLabel: string, lossTextLabel?: string): Command {
        return new WinLossTextCommand(winTextLabel, lossTextLabel);
    }

    private createEndCommand(): Command {
        return new EndCommand();
    }

    private createSpecialCommand(functionName: string): Command {
        return new SpecialCommand(functionName);
    }

    private createCallAsmCommand(label: string): Command {
        return new CallAsmCommand(label);
    }

    private createFollowCommand(leaderId: string, followerId: string): Command {
        return new FollowCommand(leaderId, followerId);
    }

    private createStopFollowCommand(): Command {
        return new StopFollowCommand();
    }

    private createLockCommand(): Command {
        return new LockCommand();
    }

    private createReleaseCommand(): Command {
        return new ReleaseCommand();
    }

    private createLockAllCommand(): Command {
        return new LockAllCommand();
    }

    private createReleaseAllCommand(): Command {
        return new ReleaseAllCommand();
    }

    private createStopCommand(): Command {
        return new StopCommand();
    }

    private createPlaySoundCommand(soundId: string): Command {
        return new PlaySoundCommand(soundId);
    }

    private createWaitSFXCommand(): Command {
        return new WaitSFXCommand();
    }

    private createPauseCommand(frames: string): Command {
        return new PauseCommand(parseScriptInt(frames));
    }

    private createEarthquakeCommand(param: string): Command {
        const { intensity, duration } = decodeEarthquakeParam(param);
        return new EarthquakeCommand(intensity, duration);
    }

    private createCryCommand(speciesName: string): Command {
        return new CryCommand(speciesName);
    }

    private createPokePicCommand(speciesName: string): Command {
        return new PokePicCommand(speciesName);
    }

    private createClosePokePicCommand(): Command {
        return new ClosePokePicCommand();
    }

    private createGetMonNameCommand(bufferName: string, speciesName: string): Command {
        return new GetMonNameCommand(bufferName, speciesName);
    }

    private createAppearCommand(objectId: string): Command {
        return new AppearCommand(objectId);
    }

    private createDisappearCommand(objectId: string): Command {
        return new DisappearCommand(objectId);
    }

    private createCheckPokeMailCommand(mailLabel: string): Command {
        if (!mailLabel || !String(mailLabel).trim()) {
            throw new Error("checkpokemail requires a mail message label.");
        }
        return new CheckPokeMailCommand(String(mailLabel).trim());
    }

    private createGivePokeMailCommand(mailLabel: string): Command {
        if (!mailLabel || !String(mailLabel).trim()) {
            throw new Error("givepokemail requires a mail definition label.");
        }
        return new GivePokeMailCommand(String(mailLabel).trim());
    }

    private createCheckTimeCommand(period: string): Command {
        return new CheckTimeCommand(period);
    }

    private createMoveObjectCommand(objectId: string, mapX: string, mapY: string): Command {
        return new MoveObjectCommand(objectId, mapX, mapY);
    }

    private createChangeBlockCommand(blockX: string, blockY: string, blockId: string): Command {
        return new ChangeBlockCommand(
            parseScriptInt(blockX),
            parseScriptInt(blockY),
            parseScriptInt(blockId),
        );
    }

    private createRefreshMapCommand(): Command {
        return new RefreshMapCommand();
    }

    private createWarpCommand(mapConstant: string, mapX: string, mapY: string): Command {
        return new WarpCommand(mapConstant, parseScriptInt(mapX), parseScriptInt(mapY));
    }

    private createWarpFacingCommand(direction: string, mapConstant: string, mapX: string, mapY: string): Command {
        return new WarpFacingCommand(direction, mapConstant, parseScriptInt(mapX), parseScriptInt(mapY));
    }

    private createWarpCheckCommand(): Command {
        return new WarpCheckCommand();
    }

    private createWarpSoundCommand(): Command {
        return new WarpSoundCommand();
    }

    private createNewLoadMapCommand(entryMethod: string): Command {
        return new NewLoadMapCommand(entryMethod);
    }

    private createSwarmCommand(swarmToken: string, mapConstant: string): Command {
        return new SwarmCommand(swarmToken, mapConstant);
    }

    private createElevatorCommand(dataLabel: string): Command {
        return new ElevatorCommand(dataLabel);
    }

    private createTurnObjectCommand(objectId: string, direction: string): Command {
        return new TurnObjectCommand(objectId, direction);
    }

    private createFaceObjectCommand(sourceObjectId: string, targetObjectId: string): Command {
        return new FaceObjectCommand(sourceObjectId, targetObjectId);
    }

    private createShowEmoteCommand(emoteId: string, objectId: string, duration: string): Command {
        return new ShowEmoteCommand(emoteId, objectId, parseScriptInt(duration));
    }

    private createApplyMovementCommand(objectId: string, movementDataLabel: string): Command {
        return new ApplyMovementCommand(objectId.replace(/,$/, ""), movementDataLabel);
    }

    private createApplyMovementLastTalkedCommand(movementDataLabel: string): Command {
        return new ApplyMovementLastTalkedCommand(movementDataLabel);
    }

    private createFacePlayerCommand(): Command {
        return new FacePlayerCommand();
    }

    private createSetMapSceneCommand(mapName: string, sceneName: string): Command {
        return new SetMapSceneCommand(mapName, sceneName);
    }

    private createCheckSceneCommand(): Command {
        return new CheckSceneCommand();
    }

    private createClearFlagCommand(flagName: string): Command {
        return new ClearFlagCommand(flagName);
    }

    private createSetFlagCommand(flagName: string): Command {
        return new SetFlagCommand(flagName);
    }

    private createSetEngineFlagCommand(flagName: string): Command {
        return new SetEngineFlagCommand(flagName);
    }

    private createVariableSpriteCommand(spriteIdentifier: string, replacementSprite: string): Command {
        return new VariableSpriteCommand(spriteIdentifier, replacementSprite);
    }

    private createDescribeDecorationCommand(descriptor: string): Command {
        return new DescribeDecorationCommand(descriptor);
    }

    private createCheckFlagCommand(flagName: string, scriptIfTrue?: string, scriptIfFalse?: string): Command {
        return new CheckFlagCommand(flagName, scriptIfTrue, scriptIfFalse);
    }

    private createConditionalEventCommand(eventName: string, scriptName: string): Command {
        return new ConditionalEventCommand(eventName, scriptName);
    }

    private createSetEventCommand(eventName: string): Command {
        return new SetEventCommand(eventName);
    }

    private createCheckEventCommand(eventName: string): Command {
        return new CheckEventCommand(eventName);
    }

    private createSpecialPhoneCallCommand(callId: string): Command {
        return new SpecialPhoneCallCommand(callId);
    }

    private createIfTrueCommand(scriptName: string): Command {
        return new IfTrueCommand(scriptName);
    }

    private createIfFalseCommand(scriptName: string): Command {
        return new IfFalseCommand(scriptName);
    }

    private createSetSceneCommand(sceneName: string): Command {
        return new SetSceneCommand(sceneName);
    }

    private createClearEventCommand(eventName: string): Command {
        return new ClearEventCommand(eventName);
    }

    private createBlackoutModCommand(mapConstant: string): Command {
        return new BlackoutModCommand(mapConstant);
    }

    private createSetLastTalkedCommand(objectId: string): Command {
        return new SetLastTalkedCommand(objectId);
    }

    private createEndIfJustBattledCommand(): Command {
        return new EndIfJustBattledCommand();
    }

    private createIfEqualCommand(value: string, scriptName: string): Command {
        return new IfEqualCommand(value, scriptName);
    }

    private createIfNotEqualCommand(value: string, scriptName: string): Command {
        return new IfNotEqualCommand(value, scriptName);
    }

    private createIfGreaterCommand(value: string, scriptName: string): Command {
        return new IfGreaterCommand(value, scriptName);
    }

    private createIfLessCommand(value: string, scriptName: string): Command {
        return new IfLessCommand(value, scriptName);
    }

    private createJumpStandardCommand(scriptName: string): Command {
        return new JumpStandardCommand(scriptName);
    }

    private createVerboseGiveItemCommand(itemName: string): Command {
        return new VerboseGiveItemCommand(itemName);
    }

    private createVerboseGiveItemVarCommand(itemName: string, quantityVar: string): Command {
        return new VerboseGiveItemVarCommand(itemName, quantityVar);
    }

    private createGivePokeCommand(speciesName: string, level: string, ...args: string[]): Command {
        const itemName = args.length ? args[0] : "NO_ITEM";
        const nicknameLabel = args.length > 1 ? args[1] : null;
        const otLabel = args.length > 2 ? args[2] : null;
        return new GivePokeCommand(speciesName, parseScriptInt(level), itemName, nicknameLabel, otLabel);
    }

    private createGiveEggCommand(speciesName: string, levelToken: string): Command {
        return new GiveEggCommand(speciesName, levelToken);
    }

    private createLoadWildMonCommand(speciesName: string, level: string): Command {
        return new LoadWildMonCommand(speciesName, parseScriptInt(level));
    }

    private createCatchTutorialCommand(battleType: string): Command {
        return new CatchTutorialCommand(battleType);
    }

    private createAskForPhoneNumberCommand(phoneNumber: string): Command {
        return new AskForPhoneNumberCommand(phoneNumber);
    }

    private createCheckCellNumCommand(phoneNumber: string): Command {
        return new CheckCellNumCommand(phoneNumber);
    }

    private createCheckItemCommand(itemName: string): Command {
        return new CheckItemCommand(itemName);
    }

    private createCheckCoinsCommand(...amountTokens: string[]): Command {
        return new CheckCoinsCommand(...amountTokens);
    }

    private createCheckMoneyCommand(...amountTokens: string[]): Command {
        return new CheckMoneyCommand(amountTokens[0], ...amountTokens.slice(1));
    }

    private createCheckPokeCommand(speciesName: string): Command {
        return new CheckPokeCommand(speciesName);
    }

    private createCheckVerCommand(): Command {
        return new CheckVersionCommand();
    }

    private createGiveCoinsCommand(...amountTokens: string[]): Command {
        return new GiveCoinsCommand(...amountTokens);
    }

    private createTakeCoinsCommand(...amountTokens: string[]): Command {
        return new TakeCoinsCommand(...amountTokens);
    }

    private createTakeMoneyCommand(...amountTokens: string[]): Command {
        return new TakeMoneyCommand(amountTokens[0], ...amountTokens.slice(1));
    }

    private createItemBallCommand(itemName: string): Command {
        return new ItemBallCommand(itemName);
    }

    private createHiddenItemCommand(itemName: string, eventFlag: string): Command {
        return new HiddenItemCommand(itemName, eventFlag);
    }

    private createItemNotifyCommand(): Command {
        return new ItemNotifyCommand();
    }

    private createLoadVarCommand(varName: string, value: string): Command {
        return new LoadVarCommand(varName, value);
    }

    private createReadVarCommand(varName: string): Command {
        return new ReadVarCommand(varName);
    }

    private createWriteVarCommand(varName: string): Command {
        return new WriteVarCommand(varName);
    }

    private createSetValCommand(value: string): Command {
        return new SetValCommand(value);
    }

    private createRandomCommand(upperBound: string): Command {
        return new RandomCommand(parseScriptInt(upperBound));
    }

    private createGetNumCommand(bufferName: string): Command {
        return new GetNumCommand(bufferName);
    }

    private createPlayMapMusicCommand(): Command {
        return new PlayMapMusicCommand();
    }

    private createReloadMapCommand(): Command {
        return new ReloadMapCommand();
    }

    private createReanchorMapCommand(anchor?: string): Command {
        return new ReanchorMapCommand(anchor);
    }

    private createTakeItemCommand(itemName: string): Command {
        return new TakeItemCommand(itemName);
    }

    private createGetItemNameCommand(bufferName: string, itemName: string): Command {
        return new GetItemNameCommand(bufferName, itemName);
    }

    private createGetTrainerNameCommand(bufferName: string, trainerClass: string, trainerName: string): Command {
        return new GetTrainerNameCommand(bufferName, trainerClass, trainerName);
    }

    private createGiveItemCommand(itemName: string, quantity?: string): Command {
        const parsed = quantity !== undefined ? parseScriptInt(quantity) : 1;
        return new GiveItemCommand(itemName, parsed);
    }

    private createPokemartCommand(martType: string, martIdentifier: string): Command {
        return new PokemartCommand(martType, martIdentifier);
    }

    private createAddCellNumCommand(phoneNumber: string): Command {
        return new AddCellNumCommand(phoneNumber);
    }

    private createReadMemCommand(address: string): Command {
        return new ReadMemCommand(address);
    }

    private createWriteMemCommand(address: string): Command {
        return new WriteMemCommand(address);
    }

    private createLoadMemCommand(address: string, value: string): Command {
        return new LoadMemCommand(address, value);
    }

    private createAddValCommand(value: string): Command {
        return new AddValCommand(value);
    }

    private createFruitTreeCommand(treeId: string): Command {
        return new FruitTreeCommand(treeId);
    }

    private createHallOfFameCommand(): Command {
        return new HallOfFameCommand();
    }
}
