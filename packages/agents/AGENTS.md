# Agents Package Guidance

This package should stay LLM-driven.
You should pick a single session and optimize the agent for full game quick progression/exploration. DO NOT RESTART
SESSIONS TO TEST CHANGES. Play 1 session through.
Do not restart a run just because the pace is disappointing. Keep the current run alive, resume the current
session, and improve the agent around that live session unless the user explicitly orders a restart.

## Operating Model

- The taskmaster is responsible for choosing the immediate objective from live game state.
- The player is responsible for choosing the next valid Game Boy input sequence from live game state.
- Both agents should reason from the MCP surface, recent outcomes, and current objective, not from hardcoded state machines.
- Hard rule: the runner, harness, CLI, MCP server, or middleware must never choose or execute gameplay actions as a fallback. Only an agent/model may choose Game Boy inputs.
- On a silent, stalled, timed-out, or no-progress model turn, record the failure, append/propagate `YOU MUST MAKE A CHOICE` into the next agentic cycle if useful, and continue to the next model turn. Do not synthesize `move`, `press`, `hold_button`, `wait`, or macro calls in code.
- Runtime behavior must match the ASM-derived source of truth or fail immediately.
- If live state, assets, prompts, or tool outputs are insufficient to preserve ASM-faithful behavior, stop with a hard failure instead of guessing.

## Do Not Build Brittle Control Logic

- Do not add regex-driven gameplay policy.
- Do not add status-string pattern matching that hardcodes specific button responses.
- Do not add brittle middleware that maps prompt/menu/dialogue states to fixed inputs outside the model.
- Do not patch local-model failures by adding screen-specific prompt instructions, synthetic render helpers, or agent-side workarounds for MCP deficiencies. Fix missing or inconsistent state at the MCP source, fix request-shape problems in the model harness, or accept that a small model may fail.
- Do not turn the agent loop into a handcrafted rules engine for full-game scripts.
- Do not add shims to smooth over missing ASM behavior.
- Do not add legacy paths to preserve superseded behavior.
- Do not add compatibility branches that silently reinterpret incorrect state or output.
- Do not fabricate success summaries, synthetic progress, or fallback decisions when the real result is missing.
- Do not add anti-stuck, recovery, unstuck, route-following, or watchdog code that presses buttons or moves the player outside the model's own tool calls.
- Browser/runtime asset URLs must use the canonical `/assets/...` path only. Do not introduce or reference `/api/assets/...` in client/runtime code, prompts, tools, or tests except inside the server route implementation itself.

## What To Prefer Instead

- Give the taskmaster a strong system prompt that states the progression priorities clearly.
- Give the player a strong system prompt that explains the honest action surface and how to reason from live state.
- Expose rich MCP state so the model can infer what to do next.
- Preserve recent action history so the model can avoid stale plans and repeated mistakes.
- Keep prompts focused on objectives, live evidence, and honest play constraints.
- Keep prompt policy generic enough to survive randomized or altered game layouts. Do not hardcode vanilla location names, NPC names, or scene-specific route scripts into system guidance.
- If a prompt mentions a concrete target, it should come from live flow state, live hotspot text, or the user’s explicit goal, not from baked-in map assumptions.

## Development Process

- Optimize for live gameplay results, not for pretty prompt tests.
- Use targeted tests and builds as safety checks, but do not confuse them with proof that the agent is good.
- Evaluate the agent by running it, watching runtime snapshots, reading training traces, and checking whether it is actually making story progress.
- During critical early-game sequences, watch the run on a short cadence and compare deltas instead of trusting a single snapshot.
- If the run stalls, inspect the exact live state, action history, and prompt context that produced the stall.
- Fix the smallest prompt or MCP-state issue that is most likely to improve the next live run.
- Do not create a fresh replacement run to evaluate every change. Prefer continuing or resuming the current run, then keep iterating until the behavior is genuinely decent.
- Do not stop to ask the user for tactical guidance while the run is clearly underperforming; continue improving it.
- Prefer prompt and state-surface improvements over brittle code-side control logic.
- Treat any need for a fallback, shim, compatibility branch, or synthetic success path as a bug to remove, not a safety feature to preserve.
- When behavior diverges from ASM expectations, fail hard at the point of divergence so the defect is visible and fixable.

## Progression Performance Goal

- The agent should reach the current full-game progression objective aggressively.
- Target: reach the next major story-critical hotspot in roughly 20 steps when the route is clean.
- Reasonable upper bound: reach that hotspot by about 50 steps.
- After reaching it, the agent should continue directly through the associated reward or unlock flow without dithering.

## Input Constraints

- Use only authentic Game Boy-valid inputs: d-pad, A, B, Start, Select, short holds, and waits.
- No macros, no teleports, no flag skips, no scripted route shortcuts, no engine-internal knowledge as a control substitute.
- Do not rely on `execute_macro` as a gameplay crutch. The player should succeed through ordinary inputs selected by the model.

## Design Bias

- Keep the policy simple, explicit, and prompt-driven.
- Improve MCP state quality when the model lacks enough information.
- Improve prompts when the model is under-directed.
- Prefer generic reasoning patterns like hotspot following, forced-interaction cleanup, and route continuity over named-scene instructions.
- Only add code-side control constraints when they are generic safety constraints, not brittle content-specific scripting.
- No shims.
- No legacy behavior.
- No compatibility layers.
- No graceful degradation.
- Either it works like the ASM or it fails hard.
