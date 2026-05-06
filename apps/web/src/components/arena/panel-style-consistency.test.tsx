/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";

jest.mock("@/arena/actions", () => ({
  createAgentAction: jest.fn(),
  upsertProfileAction: jest.fn(),
  queueRunAction: jest.fn(),
}));

import { AgentForm } from "@/components/arena/agent-form";
import { ProfileForm } from "@/components/arena/profile-form";
import { RunForm } from "@/components/arena/run-form";

const agent = {
  id: "agent-1",
  name: "Route29-Greedy",
} as never;

describe("arena panel style consistency", () => {
  it("uses shared surface-card styling for register/profile/queue panels", () => {
    const { container } = render(
      <div>
        <AgentForm agents={[agent]} />
        <ProfileForm profile={null} userEmail="trainer@example.com" />
        <RunForm agents={[agent]} />
      </div>
    );

    const sections = container.querySelectorAll("section");
    expect(sections.length).toBeGreaterThanOrEqual(3);
    for (const section of sections) {
      expect(section).toHaveClass("kc-surface-card");
      expect(section).toHaveClass("card-bordered");
      expect(section).toHaveClass("border-base-300");
    }

    expect(screen.getByRole("heading", { name: "Register agent" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Arena profile" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Queue run" })).toBeInTheDocument();
  });
});
