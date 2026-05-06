/**
 * Shared glyph constants matching pokecrystal_disassembly/constants/charmap.asm.
 * Private-use glyphs mirror the Python renderer's placeholders.
 */

export const POKE_GLYPH = "#";
export const POKEMON_WORD = `${POKE_GLYPH}MON`;

export const PKMN_GLYPH = "\ue100";
export const PC_GLYPH = "\ue101";
export const TM_GLYPH = "\ue102";
export const TRAINER_GLYPH = "\ue103";
export const ROCKET_GLYPH = "\ue104";
export const PK_GLYPH = "\ue105";
export const MN_GLYPH = "\ue106";
export const DOT_GLYPH = "\ue107";
export const PO_GLYPH = "\ue108";
export const KE_GLYPH = "\ue109";
export const LV_GLYPH = "\ue10a";
export const ID_GLYPH = "\ue10b";

export const SIX_DOTS_TEXT = "……";

export const CONTROL_CODE_REPLACEMENTS: ReadonlyArray<[string, string]> = [
  ["<TRAINER>", TRAINER_GLYPH],
  ["<ROCKET>", ROCKET_GLYPH],
  ["<PKMN>", `${PK_GLYPH}${MN_GLYPH}`],
  ["<POKE>", POKE_GLYPH],
  ["<PC>", PC_GLYPH],
  ["<TM>", TM_GLYPH],
  ["<PK>", PK_GLYPH],
  ["<MN>", MN_GLYPH],
  ["<DOT>", DOT_GLYPH],
  ["<PO>", PO_GLYPH],
  ["<KE>", KE_GLYPH],
  ["<LV>", LV_GLYPH],
  ["<ID>", ID_GLYPH],
  ["<……>", SIX_DOTS_TEXT],
];

export function applyTextReplacements(
  text: string,
  replacements: ReadonlyArray<[string, string]> = CONTROL_CODE_REPLACEMENTS
): string {
  let normalized = text;
  for (const [token, replacement] of replacements) {
    normalized = normalized.split(token).join(replacement);
  }
  // Mirror the Python renderer: expand the Poké ligature into its component tiles.
  return normalized.split(POKE_GLYPH).join("POK\u00e9");
}
