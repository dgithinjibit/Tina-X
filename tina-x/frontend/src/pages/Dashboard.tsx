import { useState } from 'react';
import { motion } from 'framer-motion';
import { ArrowLeft } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { HazardSelector } from '../components/HazardSelector';
import { CascadeViewer } from '../components/CascadeViewer';
import { InfraGraph } from '../components/InfraGraph';
import { AlertPanel } from '../components/AlertPanel';
import { simulate } from '../api';
import type { SimulationResult } from '../types';

export function Dashboard() {
  const navigate = useNavigate();
  const [result, setResult] = useState<SimulationResult | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSimulate = async (scenarioKey: string) => {
    setLoading(true);
    try {
      const res = await simulate(scenarioKey);
      setResult(res);
    } catch (err) {
      console.error('Simulation failed:', err);
      alert('Simulation failed. Check console for details.');
    } finally {
      setLoading(false);
    }
  };

  const failedNodes = result?.failures.map((f) => f.node) || [];

  return (
    <div className="min-h-screen p-4 md:p-6">
      {/* Header */}
      <motion.div
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        className="mb-6 flex items-center justify-between"
      >
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate('/')}
            className="btn-cyan text-sm"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
          <div>
            <h1 className="text-3xl font-bold">
              <span className="text-neon-cyan">TINA-X</span>{' '}
              <span className="text-cyber-muted text-xl">Control Center</span>
            </h1>
            <p className="text-xs text-cyber-muted mt-1">
              Symbolic cascading failure analysis — real-time digital twin
            </p>
          </div>
        </div>
        
        {loading && (
          <div className="status-warning animate-pulse">
            SIMULATING...
          </div>
        )}
      </motion.div>

      {/* Dashboard Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 h-[calc(100vh-140px)]">
        {/* Left Column */}
        <motion.div
          initial={{ opacity: 0, x: -20 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ delay: 0.1 }}
          className="space-y-4"
        >
          <HazardSelector onSelectScenario={handleSimulate} loading={loading} />
          <AlertPanel />
        </motion.div>

        {/* Middle Column */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="lg:col-span-1"
        >
          <CascadeViewer result={result} />
        </motion.div>

        {/* Right Column */}
        <motion.div
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ delay: 0.3 }}
          className="lg:col-span-1"
        >
          <InfraGraph highlightNodes={failedNodes} />
        </motion.div>
      </div>
    </div>
  );
}
