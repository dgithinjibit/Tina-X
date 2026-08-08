// Component tests for the brain-decision panel (render behavior in jsdom).
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { BrainDecisionPanel } from "./BrainDecisionPanel";
import type { BrainDecision } from "../useTelemetry";

const decisions: BrainDecision[] = [
  { id: 2, query: "!(safe? 8)", results: ["True"], verified: true },
  { id: 1, query: "!(treat? weed-3)", results: [], verified: false },
];

describe("BrainDecisionPanel", () => {
  it("shows an honest empty state when there are no decisions", () => {
    render(<BrainDecisionPanel decisions={[]} />);
    expect(screen.getByTestId("brain-empty")).toBeInTheDocument();
  });

  it("renders each decision with its query and verification flag", () => {
    render(<BrainDecisionPanel decisions={decisions} />);
    expect(screen.getAllByTestId("brain-item")).toHaveLength(2);
    expect(screen.getByText("!(safe? 8)")).toBeInTheDocument();
    // Verified/unverified flags render distinctly.
    const flags = screen.getAllByTestId("brain-verified").map((n) => n.textContent);
    expect(flags).toContain("✓ verified");
    expect(flags).toContain("✗ unverified");
  });

  it("shows '(no results)' when a decision returned nothing", () => {
    render(<BrainDecisionPanel decisions={decisions} />);
    expect(screen.getByText(/\(no results\)/)).toBeInTheDocument();
  });
});
