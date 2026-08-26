// Component tests for the BOM table (render behavior in jsdom).
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { BomTable } from "./BomTable";
import type { Bom } from "../api";

const fakeBom: Bom = {
  items: [
    { name: "MEMS rate gyro", purpose: "reflex sensing", rationale: "fly halteres", est_cost_usd: 3, qty: 1 },
    { name: "Motors", purpose: "actuation", rationale: "cheap+replaceable", est_cost_usd: 6, qty: 4 },
  ],
  total_est_cost_usd: 27,
};

describe("BomTable", () => {
  it("shows a loading state when bom is null and no error", () => {
    render(<BomTable bom={null} />);
    expect(screen.getByTestId("bom-loading")).toBeInTheDocument();
  });

  it("shows a helpful error when the request failed", () => {
    render(<BomTable bom={null} error="500" />);
    expect(screen.getByTestId("bom-error")).toHaveTextContent("cargo run -p tina-server");
  });

  it("renders every item and the total", () => {
    render(<BomTable bom={fakeBom} />);
    expect(screen.getByText("MEMS rate gyro")).toBeInTheDocument();
    expect(screen.getByText("Motors")).toBeInTheDocument();
    // Motors line total = 6 * 4 = $24.00
    expect(screen.getByText("$24.00")).toBeInTheDocument();
    expect(screen.getByTestId("bom-total")).toHaveTextContent("$27.00");
  });
});
