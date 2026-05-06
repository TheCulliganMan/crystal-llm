export type OverworldDialogue = {
  active?: boolean;
  waiting_for_input?: boolean;
  handle_input?: (event: unknown) => void;
  update?: () => void;
  draw?: () => void;
  window?: { complete?: () => void; is_complete?: () => boolean } | null;
  pendingWaits?: number;
  pending_script_waits?: number;
  script_runner?: { resume?: () => void } | null;
  script_paused?: boolean;
  _suppress_orphan_close?: boolean;
};
