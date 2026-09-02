import { motion } from 'framer-motion';
import { useNavigate } from 'react-router-dom';
import { Zap, Brain, Shield, Activity } from 'lucide-react';

export function Landing() {
  const navigate = useNavigate();

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-6 scanlines">
      <motion.div
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        className="text-center max-w-4xl"
      >
        {/* Hero */}
        <motion.div
          initial={{ scale: 0.9 }}
          animate={{ scale: 1 }}
          transition={{ duration: 0.5 }}
          className="mb-8"
        >
          <h1 className="text-6xl font-bold mb-4">
            <span className="text-neon-cyan animate-glow">TINA</span>
            <span className="text-neon-magenta">-X</span>
          </h1>
          <p className="text-2xl text-neon-green uppercase tracking-widest font-bold">
            Cascading Failure Reasoner
          </p>
        </motion.div>

        {/* Tagline */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.3 }}
          className="mb-12 text-cyber-text text-lg max-w-2xl mx-auto"
        >
          <p className="mb-2">
            Digital Twin of Society's Fragility
          </p>
          <p className="text-cyber-muted text-sm">
            Symbolic AI reasoning about out-of-distribution black swan events — 
            where deep learning fails, MeTTa prevails.
          </p>
        </motion.div>

        {/* Feature Grid */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.5 }}
          className="grid md:grid-cols-2 lg:grid-cols-4 gap-4 mb-12"
        >
          {[
            { icon: Brain, title: 'Symbolic', desc: 'MeTTa logic engine' },
            { icon: Zap, title: 'Real-time', desc: 'Live cascade detection' },
            { icon: Shield, title: 'Resilience', desc: 'Infrastructure hardening' },
            { icon: Activity, title: 'Explainable', desc: 'Trace every failure' },
          ].map((feat, idx) => (
            <motion.div
              key={feat.title}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.6 + idx * 0.1 }}
              className="panel-glow p-4 text-left"
            >
              <feat.icon className="w-8 h-8 text-neon-cyan mb-2" />
              <div className="text-neon-green font-bold uppercase text-sm">
                {feat.title}
              </div>
              <div className="text-cyber-muted text-xs mt-1">{feat.desc}</div>
            </motion.div>
          ))}
        </motion.div>

        {/* CTA */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 1 }}
          className="space-y-4"
        >
          <button
            onClick={() => navigate('/dashboard')}
            className="btn-cyan text-lg px-8 py-3"
          >
            Launch Dashboard
          </button>
          
          <div className="text-xs text-cyber-muted">
            Part of the{' '}
            <span className="text-neon-magenta font-bold">Project-Nzi</span>{' '}
            ecosystem — two-rate brain architecture for disaster response
          </div>
        </motion.div>

        {/* Tech Stack Badge */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 1.2 }}
          className="mt-12 pt-8 border-t border-cyber-border"
        >
          <div className="text-xs text-cyber-muted uppercase mb-2">Powered By</div>
          <div className="flex flex-wrap justify-center gap-3 text-xs">
            {['MeTTa', 'Python', 'FastAPI', 'React', 'TypeScript', 'Tailwind', 'Framer Motion'].map((tech) => (
              <span key={tech} className="px-2 py-1 border border-cyber-border rounded text-cyber-text">
                {tech}
              </span>
            ))}
          </div>
        </motion.div>
      </motion.div>

      {/* Gen-X Easter Egg */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 0.3 }}
        transition={{ delay: 2 }}
        className="absolute bottom-4 right-4 text-[10px] text-cyber-muted font-mono"
      >
        EST. 2026 // GEN-X AESTHETIC // CYBERPUNK NOIR
      </motion.div>
    </div>
  );
}
