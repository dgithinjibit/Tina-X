import { useEffect, useState } from 'react';
import { Bell, Info, AlertTriangle, XCircle } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import type { Alert } from '../types';
import { getAlerts } from '../api';

export function AlertPanel() {
  const [alerts, setAlerts] = useState<Alert[]>([]);

  useEffect(() => {
    const interval = setInterval(() => {
      getAlerts()
        .then((res) => setAlerts(res.alerts))
        .catch(() => {});
    }, 3000);

    return () => clearInterval(interval);
  }, []);

  const getIcon = (level: Alert['level']) => {
    switch (level) {
      case 'critical':
        return <XCircle className="w-4 h-4" />;
      case 'warning':
        return <AlertTriangle className="w-4 h-4" />;
      default:
        return <Info className="w-4 h-4" />;
    }
  };

  const getStyle = (level: Alert['level']) => {
    switch (level) {
      case 'critical':
        return 'bg-status-critical/10 border-status-critical/30 text-status-critical';
      case 'warning':
        return 'bg-status-warning/10 border-status-warning/30 text-status-warning';
      default:
        return 'bg-blue-500/10 border-blue-500/30 text-blue-400';
    }
  };

  return (
    <div className="panel-glow">
      <div className="flex items-center gap-2 mb-4">
        <Bell className="w-5 h-5 text-neon-magenta" />
        <h2 className="text-xl text-neon-magenta uppercase tracking-wider">Live Alerts</h2>
      </div>

      {alerts.length === 0 ? (
        <div className="text-center text-cyber-muted text-sm py-4">
          No alerts — system nominal
        </div>
      ) : (
        <div className="space-y-2 max-h-64 overflow-auto">
          <AnimatePresence>
            {alerts.map((alert, idx) => (
              <motion.div
                key={idx}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: 20 }}
                className={`p-3 rounded border ${getStyle(alert.level)}`}
              >
                <div className="flex items-start gap-2">
                  {getIcon(alert.level)}
                  <div className="flex-1 text-sm">{alert.message}</div>
                  {alert.timestamp && (
                    <div className="text-xs opacity-70">{new Date(alert.timestamp).toLocaleTimeString()}</div>
                  )}
                </div>
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
}
