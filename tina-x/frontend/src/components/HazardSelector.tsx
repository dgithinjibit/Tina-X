import { useState, useEffect } from 'react';
import { AlertCircle } from 'lucide-react';
import type { Scenario } from '../types';
import { getScenarios, MOCK_SCENARIOS } from '../api';

interface Props {
  onSelectScenario: (key: string) => void;
  loading: boolean;
}

export function HazardSelector({ onSelectScenario, loading }: Props) {
  const [scenarios, setScenarios] = useState<Scenario[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [apiAvailable, setApiAvailable] = useState(true);

  useEffect(() => {
    getScenarios()
      .then((res) => setScenarios(res.scenarios))
      .catch(() => {
        setApiAvailable(false);
        setScenarios(MOCK_SCENARIOS);
      });
  }, []);

  const handleSelect = (key: string) => {
    setSelected(key);
    onSelectScenario(key);
  };

  return (
    <div className="panel-glow">
      <div className="flex items-center gap-2 mb-4">
        <AlertCircle className="w-5 h-5 text-neon-yellow" />
        <h2 className="text-xl text-neon-yellow uppercase tracking-wider">
          Hazard Scenarios
        </h2>
      </div>

      {!apiAvailable && (
        <div className="mb-4 p-2 bg-status-warning/10 border border-status-warning/30 rounded text-xs text-status-warning">
          API offline — using mock data
        </div>
      )}

      <div className="space-y-3">
        {scenarios.map((scenario) => (
          <button
            key={scenario.key}
            onClick={() => handleSelect(scenario.key)}
            disabled={loading}
            className={`w-full text-left p-3 border-2 rounded transition-all ${
              selected === scenario.key
                ? 'border-neon-cyan bg-neon-cyan/10 shadow-lg shadow-neon-cyan/30'
                : 'border-cyber-border hover:border-neon-cyan/50'
            } disabled:opacity-50`}
          >
            <div className="font-bold text-neon-green">{scenario.name}</div>
            <div className="text-xs text-cyber-muted mt-1">{scenario.description}</div>
            <div className="text-xs text-cyber-text mt-2 font-mono">
              {Object.entries(scenario.changes).map(([k, v]) => (
                <span key={k} className="mr-3">
                  {k}: <span className="text-neon-magenta">{v}</span>
                </span>
              ))}
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
