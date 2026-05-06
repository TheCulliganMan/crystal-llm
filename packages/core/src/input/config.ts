import { z } from 'zod';

export enum GameButton {
    A = 'a',
    B = 'b',
    Start = 'start',
    Select = 'select',
}

const KeyBindingsSchema = z.object({
    [GameButton.A]: z.array(z.string()),
    [GameButton.B]: z.array(z.string()),
    [GameButton.Start]: z.array(z.string()),
    [GameButton.Select]: z.array(z.string()),
});

export const defaultKeyBindings: Record<GameButton, string[]> = KeyBindingsSchema.parse({
    [GameButton.A]: ['KeyZ', 'Space', 'KeyA', 'KeyJ'],
    [GameButton.B]: ['KeyX', 'KeyK'],
    [GameButton.Start]: ['Enter', 'NumpadEnter', 'KeyS'],
    [GameButton.Select]: ['Backspace', 'Escape'],
});
