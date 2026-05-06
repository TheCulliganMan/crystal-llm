"use client";

import { useEffect, useMemo, useState } from "react";
import mermaid from "mermaid";
import DOMPurify from "dompurify";
import {
  STORY_STEPS,
  buildMermaidDiagram,
  calculateCompletionPercent,
  getAvailableStepIds,
  validateStoryGraph,
} from "@/app/game-corner/progress-tracker";

const mermaidReady = mermaid.initialize({
  startOnLoad: false,
  securityLevel: "strict",
  theme: "base",
  flowchart: {
    htmlLabels: false,
  },
  themeVariables: {
    background: "#f5efe5",
    primaryColor: "#f2e6d5",
    primaryBorderColor: "#7c4a21",
    primaryTextColor: "#1f2a38",
    lineColor: "#6b7280",
    textColor: "#1f2a38",
    nodeBorder: "#7c4a21",
    clusterBkg: "#ebe1d3",
    clusterBorder: "#b89b79",
    tertiaryColor: "#e3d6c4",
    edgeLabelBackground: "#f5efe5",
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, Liberation Mono, Courier New, monospace",
  },
});
void mermaidReady;

const GRAPH_VALIDATION = validateStoryGraph(STORY_STEPS);
if (!GRAPH_VALIDATION.isAcyclic || GRAPH_VALIDATION.hasDanglingReferences || !GRAPH_VALIDATION.isRedReachable) {
  throw new Error(
    `Invalid progress graph: acyclic=${GRAPH_VALIDATION.isAcyclic} dangling=${GRAPH_VALIDATION.hasDanglingReferences} redReachable=${GRAPH_VALIDATION.isRedReachable}`,
  );
}

export const ProgressTrackerPanel = () => {
  const [completedStepIds, setCompletedStepIds] = useState<string[]>([]);
  const [svg, setSvg] = useState<string>("");

  const diagramText = useMemo(
    () => buildMermaidDiagram(STORY_STEPS, completedStepIds),
    [completedStepIds],
  );

  const completionPercent = useMemo(
    () => calculateCompletionPercent(STORY_STEPS, completedStepIds),
    [completedStepIds],
  );

  const availableStepIds = useMemo(
    () => new Set(getAvailableStepIds(STORY_STEPS, completedStepIds)),
    [completedStepIds],
  );

  useEffect(() => {
    let active = true;

    const render = async () => {
      const renderId = `story-progress-${completedStepIds.join("-") || "none"}`;
      const { svg: renderedSvg } = await mermaid.render(renderId, diagramText);
      if (active) {
        setSvg(DOMPurify.sanitize(renderedSvg));
      }
    };

    void render();

    return () => {
      active = false;
    };
  }, [completedStepIds, diagramText]);

  const toggleStep = (stepId: string) => {
    setCompletedStepIds((current) => {
      if (current.includes(stepId)) {
        return current.filter((id) => id !== stepId);
      }

      const available = new Set(getAvailableStepIds(STORY_STEPS, current));
      if (!available.has(stepId)) {
        return current;
      }

      return [...current, stepId];
    });
  };

  return (
    <section className="space-y-4" data-testid="game-corner-progress-tracker">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="space-y-1">
          <h3 className="text-base font-semibold text-base-content">Crystal Completion Planner</h3>
          <p className="max-w-3xl text-sm leading-6 text-base-content/80">
            ASM-linked mandatory story gates from New Bark Town to Red. Only currently reachable milestones can be checked.
          </p>
          <p className="text-xs font-medium text-base-content/75">Progress: {completionPercent}% complete</p>
        </div>
        <a className="btn btn-xs btn-outline" href="/downloads/krabbyclaw-progress-tracker-skill.zip" download>
          Download Progress Tracker Skill
        </a>
      </div>

      <div className="grid gap-4 lg:grid-cols-[minmax(280px,400px)_minmax(0,1fr)]">
        <ul className="space-y-2 rounded-lg border border-base-300 bg-base-100/65 p-3 shadow-sm">
          {STORY_STEPS.map((step) => {
            const isDone = completedStepIds.includes(step.id);
            const isAvailable = availableStepIds.has(step.id);
            const isDisabled = !isDone && !isAvailable;
            return (
              <li key={step.id} className="rounded-md border border-base-300/80 bg-base-100/85 px-2 py-2 shadow-sm">
                <label className={`flex items-start gap-2 text-sm ${isDisabled ? "cursor-not-allowed opacity-80" : "cursor-pointer"}`}>
                  <input
                    type="checkbox"
                    className="checkbox checkbox-sm mt-0.5"
                    checked={isDone}
                    onChange={() => toggleStep(step.id)}
                    disabled={isDisabled}
                    aria-label={`Mark ${step.title} complete`}
                  />
                  <span className="min-w-0">
                    <span className="block font-medium text-base-content">{step.title}</span>
                    <span className="block text-xs leading-5 text-base-content/75">{step.description}</span>
                    <span className="block text-[11px] font-mono leading-5 text-base-content/65">Route: {step.mapRoute}</span>
                    <span className="block text-[11px] font-mono leading-5 text-base-content/65">ASM: {step.asmLabel}</span>
                    <span className="block text-[11px] font-mono leading-5 text-base-content/65">setflag: {step.setFlag}</span>
                    <span className="block text-[11px] font-mono leading-5 text-base-content/65">applymovement: {step.applyMovement}</span>
                  </span>
                </label>
              </li>
            );
          })}
        </ul>

        <div className="space-y-2">
          <div className="overflow-x-auto rounded-lg border border-base-300 bg-base-100/95 p-3 shadow-sm">
            {svg ? (
              <div className="min-w-[900px] [&_svg]:h-auto [&_svg]:w-full [&_svg_text]:fill-base-content [&_.edgeLabel]:text-base-content" dangerouslySetInnerHTML={{ __html: svg }} />
            ) : (
              <p className="p-3 text-sm text-base-content/75">Rendering Mermaid diagram...</p>
            )}
          </div>
          <details className="rounded-lg border border-base-300 bg-base-100/80 p-2 shadow-sm">
            <summary className="cursor-pointer text-sm font-medium">Agent export (Mermaid source)</summary>
            <pre className="mt-2 max-h-48 overflow-auto rounded border border-base-300/70 bg-base-200/70 p-2 text-xs text-base-content">{diagramText}</pre>
          </details>
        </div>
      </div>
    </section>
  );
};
