import { STORY_STEPS, type StoryStep } from "@/app/game-corner/progress-tracker";

export type McpFlowMilestone = {
  id: string;
  title: string;
  description?: string;
  map_route?: string;
  spoiler_masked?: boolean;
  completed?: boolean;
  available?: boolean;
};

export type McpFlowStateSnapshot = {
  completion_target: {
    id: string;
    title: string;
  };
  summary: string;
  completed_count: number;
  total_count: number;
  completed: McpFlowMilestone[];
  available: McpFlowMilestone[];
  remaining: McpFlowMilestone[];
  next_goal?: McpFlowMilestone;
  remaining_path: McpFlowMilestone[];
};

type EventFlags = Record<string, boolean | undefined> | null | undefined;

const HIDDEN_STEP_IDS = new Set([
  "mahogany-rocket",
  "goldenrod-rocket",
]);

const TERMINAL_COMPLETION_ID = "mt-silver";
const TERMINAL_COMPLETION_TITLE = "Beat Mt. Silver";

const PUBLIC_ID_PREFIX = "hidden";

const isCompleted = (step: StoryStep, eventFlags: EventFlags): boolean => {
  if (!eventFlags) {
    return false;
  }
  return Boolean(eventFlags[step.setFlag]);
};

const isHidden = (step: StoryStep): boolean => HIDDEN_STEP_IDS.has(step.id);

const publicIdForStep = (step: StoryStep, index: number): string =>
  isHidden(step) ? `${PUBLIC_ID_PREFIX}-${index + 1}` : step.id;

const publicTitleForStep = (step: StoryStep): string => {
  if (step.id === "red-defeated") {
    return TERMINAL_COMPLETION_TITLE;
  }
  if (isHidden(step)) {
    return "???";
  }
  return step.title;
};

const publicDescriptionForStep = (step: StoryStep): string | undefined => {
  if (isHidden(step)) {
    return undefined;
  }
  if (step.id === "red-defeated") {
    return "Reach Mt. Silver summit and win the final battle.";
  }
  return step.description;
};

const publicRouteForStep = (step: StoryStep): string | undefined => {
  if (isHidden(step)) {
    return undefined;
  }
  if (step.id === "red-defeated") {
    return "Mt. Silver Summit";
  }
  return step.mapRoute;
};

const toMilestone = (
  step: StoryStep,
  index: number,
  eventFlags: EventFlags,
  completedSet: Set<string>
): McpFlowMilestone => {
  const completed = completedSet.has(step.id);
  const available = !completed && step.prerequisites.every((required) => completedSet.has(required));
  return {
    id: publicIdForStep(step, index),
    title: publicTitleForStep(step),
    description: publicDescriptionForStep(step),
    map_route: publicRouteForStep(step),
    spoiler_masked: isHidden(step) || undefined,
    completed: completed || undefined,
    available: available || undefined,
  };
};

export const buildFlowStateSnapshot = (eventFlags: EventFlags): McpFlowStateSnapshot => {
  const completedStepIds: string[] = [];
  for (const step of STORY_STEPS) {
    const prerequisitesMet = step.prerequisites.every((required) => completedStepIds.includes(required));
    if (!prerequisitesMet) {
      continue;
    }
    if (isCompleted(step, eventFlags)) {
      completedStepIds.push(step.id);
    }
  }
  const completedSet = new Set(completedStepIds);
  const milestones = STORY_STEPS.map((step, index) => toMilestone(step, index, eventFlags, completedSet));
  const available = milestones.filter((step) => step.available);
  const completed = milestones.filter((step) => step.completed);
  const remaining = milestones.filter((step) => !step.completed);
  const nextGoal = available[0] ?? remaining[0];
  const summary =
    remaining.length === 0
      ? `${TERMINAL_COMPLETION_TITLE} complete`
      : nextGoal
        ? `Next goal: ${nextGoal.title}`
        : "Progress unavailable";

  return {
    completion_target: {
      id: TERMINAL_COMPLETION_ID,
      title: TERMINAL_COMPLETION_TITLE,
    },
    summary,
    completed_count: completed.length,
    total_count: milestones.length,
    completed,
    available,
    remaining,
    next_goal: nextGoal,
    remaining_path: remaining,
  };
};
