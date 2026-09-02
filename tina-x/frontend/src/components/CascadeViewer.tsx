import { motion, AnimatePresence } from 'framer-motion';
import { AlertTriangle, Activity } from 'lucide-react';
import type { SimulationResult } from '../types';

interface Props {
  result: SimulationResult | null;
}

export function CascadeViewer({ result }: Props) {
  if (!result) {
    return (
      <div className="panel-glow h-full flex items-center justify-center">
        <div className="text-center text-cyber-muted">
          <Activity className="w-12 h-12 mx-auto mb-2 opacity-30" />
          <p className="text-sm">Select a scenario to simulate cascading failures</p>
        </div>
      </div>
    );
  }

  const criticalCount = result.failures.length;
  const totalNodes = Object.keys(result.statuses).length;

  return (
    <div className="panel-glow h-full overflow-auto scanlines">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <AlertTriangle className="w-5 h-5 text-neon-red" />
          <h2 className="text-xl text-neon-red uppercase tracking-wider">
            Cascade Analysis
          </h2>
        </div>
        <div className="flex gap-3 text-xs">
          <div className="status-critical">
            {criticalCount} FAILURES
          </div>
          <div className="status-badge bg-cyber-gray/20 text-cyber-text border border-cyber-gray">
            {totalNodes} NODES
          </div>
        </div>
      </div>

      <div className="mb-6 p-3 bg-cyber-darker border border-cyber-border rounded">
        <div className="text-xs text-cyber-muted uppercase mb-1">Scenario</div>
        <div className="text-neon-cyan font-bold">{result.scenario.name}</div>
        <div className="text-xs text-cyber-text mt-1">{result.scenario.description}</div>
      </div>

      <AnimatePresence>
        {criticalCount > 0 ? (
          <div className="space-y-3">
            <div className="text-xs uppercase text-neon-red font-bold mb-2">
              ⚠ Cascading Failures Detected
            </div>
            {result.failures.map((failure, idx) => (
              <motion.div
                key={`${failure.node}-${idx}`}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: idx * 0.1 }}
                className="p-3 bg-status-critical/10 border border-status-critical/30 rounded"
              >
                <div className="flex items-start gap-2">
                  <div className="text-status-critical font-mono text-xs mt-0.5">
                    [{failure.kind}]
                  </div>
                  <div className="flex-1">
                    <div className="font-bold text-neon-red">{failure.node}</div>
                    <div className="text-xs text-cyber-text mt-1">{failure.message}</div>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        ) : (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="p-4 bg-status-healthy/10 border border-status-healthy/30 rounded text-center"
          >
            <div className="text-status-healthy font-bold">✓ No Cascading Failures</div>
            <div className="text-xs text-cyber-muted mt-1">
              Infrastructure resilient to this scenario
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      <div className="mt-6">
        <div className="text-xs uppercase text-cyber-muted font-bold mb-3">
          Infrastructure Status
        </div>
        <div className="grid grid-cols-2 gap-2">
          {Object.entries(result.statuses).map(([node, status]) => (
            <div
              key={node}
              className={`p-2 rounded border text-xs ${
                status === 'offline' || status === 'failed'
                  ? 'bg-status-critical/10 border-status-critical/30 text-status-critical'
                  : status === 'degraded' || status === 'flooded' || status === 'compromised'
                  ? 'bg-status-warning/10 border-status-warning/30 text-status-warning'
                  : 'bg-status-healthy/10 border-status-healthy/30 text-status-healthy'
              }`}
            >
              <div className="font-bold font-mono">{node}</div>
              <div className="text-[10px] uppercase mt-0.5">{status}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
