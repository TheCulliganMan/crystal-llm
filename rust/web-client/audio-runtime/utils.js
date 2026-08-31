import { FRAME_SEQUENCER_RATE, SAMPLE_RATE, TICK_TO_FRAME_DENOMINATOR, TICK_TO_FRAME_NUMERATOR, } from "./constants.js";
export const sampleOffsetToFrameTick = (sampleOffset) => Math.floor((Math.max(0, sampleOffset) * FRAME_SEQUENCER_RATE) / SAMPLE_RATE);
export const frameTickToSampleOffset = (frameTick) => Math.floor((Math.max(0, frameTick) * SAMPLE_RATE) / FRAME_SEQUENCER_RATE);
export const nextFrameTickWithStepStrict = (startTick, step) => {
    const tick = Math.max(0, startTick);
    let offset = (step - (tick % 8)) % 8;
    if (offset === 0) {
        offset = 8;
    }
    return tick + offset;
};
export const next128hzTickStrict = (startTick) => {
    let offset = (3 - (Math.max(0, startTick) % 4)) % 4;
    if (offset === 0) {
        offset = 4;
    }
    return startTick + offset;
};
export const ticksToFrames = (ticks, remainder) => {
    const total = ticks * TICK_TO_FRAME_NUMERATOR + remainder;
    const frames = Math.floor(total / TICK_TO_FRAME_DENOMINATOR);
    const nextRemainder = total % TICK_TO_FRAME_DENOMINATOR;
    return [frames, nextRemainder];
};
export const regToFreqWave = (n) => {
    const clamped = Math.max(0, Math.min(2047, n));
    return 65_536.0 / (2048 - clamped);
};
export const freqToRegWave = (fHz) => {
    if (fHz <= 0) {
        return 0;
    }
    return Math.max(0, Math.min(2047, Math.round(2048 - 65_536.0 / fHz)));
};
export const parseNumber = (tokenRaw) => {
    let token = tokenRaw.trim();
    if (!token) {
        throw new Error("empty numeric token");
    }
    let sign = 1;
    if (token.startsWith("-")) {
        sign = -1;
        token = token.slice(1);
    }
    else if (token.startsWith("+")) {
        token = token.slice(1);
    }
    let value;
    if (token.toLowerCase().startsWith("0x")) {
        value = parseInt(token, 16);
    }
    else if (token.startsWith("$")) {
        value = parseInt(token.slice(1), 16);
    }
    else if (token.startsWith("%")) {
        value = parseInt(token.slice(1), 2);
    }
    else {
        value = parseInt(token, 10);
    }
    if (Number.isNaN(value)) {
        throw new Error(`invalid numeric token: ${tokenRaw}`);
    }
    return sign * value;
};
