import { z } from 'zod';

/** Mirror a Day Care control byte from the ASM memory layout. */
export const DayCareControlRegisterSchema = z.object({
  value: z.number().int().default(0),
});
export type DayCareControlRegister = z.infer<
  typeof DayCareControlRegisterSchema
>;

export const ACTIVE_MASK = 0x80;
export const EGG_READY_MASK = 0x40;
export const COMPATIBLE_MASK = 0x20;
export const MONSTER_PRESENT_MASK = 0x01;

function _set_flag(
  register: DayCareControlRegister,
  mask: number,
  enabled: boolean
): void {
  if (enabled) {
    register.value |= mask;
  } else {
    register.value &= ~mask;
  }
}

/** Track whether the caretakers are currently available. */
export function setActive(
  register: DayCareControlRegister,
  active: boolean
): void {
  _set_flag(register, ACTIVE_MASK, active);
}

/** Set the egg-ready indicator that the Day Care Man uses. */
export function setEggReady(
  register: DayCareControlRegister,
  ready: boolean
): void {
  _set_flag(register, EGG_READY_MASK, ready);
}

/** Store whether the stored Pokémon are currently mating-compatible. */
export function setMonstersCompatible(
  register: DayCareControlRegister,
  compatible: boolean
): void {
  _set_flag(register, COMPATIBLE_MASK, compatible);
}

/** Toggle the bit that marks whether a slot contains a Pokémon. */
export function setMonsterPresent(
  register: DayCareControlRegister,
  present: boolean
): void {
  _set_flag(register, MONSTER_PRESENT_MASK, present);
}
