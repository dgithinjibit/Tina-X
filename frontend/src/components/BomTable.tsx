// Renders the IoT bill of materials. This is the "it's real hardware" evidence for judges/YC.

import type { Bom } from "../api";

interface Props {
  bom: Bom | null;
  error?: string | null;
}

export function BomTable({ bom, error }: Props) {
  if (error) {
    return (
      <p data-testid="bom-error" style={{ color: "#f87171" }}>
        Could not load BOM ({error}). Is the Rust server running? (`cargo run -p tina-server`)
      </p>
    );
  }
  if (!bom) {
    return <p data-testid="bom-loading">Loading bill of materials…</p>;
  }

  return (
    <div>
      <table data-testid="bom-table" style={{ width: "100%", borderCollapse: "collapse" }}>
        <thead>
          <tr style={{ textAlign: "left", borderBottom: "1px solid #23304d" }}>
            <th>Component</th>
            <th>Purpose</th>
            <th>Why (fly / research)</th>
            <th style={{ textAlign: "right" }}>Qty</th>
            <th style={{ textAlign: "right" }}>Est. $</th>
          </tr>
        </thead>
        <tbody>
          {bom.items.map((it) => (
            <tr key={it.name} style={{ borderBottom: "1px solid #16203a" }}>
              <td>{it.name}</td>
              <td style={{ color: "#9fb3d1" }}>{it.purpose}</td>
              <td style={{ color: "#6b7fa3", fontSize: "0.85em" }}>{it.rationale}</td>
              <td style={{ textAlign: "right" }}>{it.qty}</td>
              <td style={{ textAlign: "right" }}>${(it.est_cost_usd * it.qty).toFixed(2)}</td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          <tr style={{ fontWeight: 700 }}>
            <td colSpan={4} style={{ textAlign: "right" }}>
              Estimated total per agent:
            </td>
            <td data-testid="bom-total" style={{ textAlign: "right" }}>
              ${bom.total_est_cost_usd.toFixed(2)}
            </td>
          </tr>
        </tfoot>
      </table>
      <p style={{ color: "#6b7fa3", fontSize: "0.85em", marginTop: 8 }}>
        Rough per-unit estimates for reasoning about a low-cost swarm — not procurement quotes.
      </p>
    </div>
  );
}
