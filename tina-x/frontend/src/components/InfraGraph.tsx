import { useEffect, useState } from 'react';
import { ReactFlow, Background, Controls, useNodesState, useEdgesState } from '@xyflow/react';
import type { Node, Edge } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { Network } from 'lucide-react';
import type { InfraGraph } from '../types';
import { getGraph, MOCK_GRAPH } from '../api';

interface Props {
  highlightNodes?: string[];
}

const NODE_TYPE_COLORS: Record<string, string> = {
  PowerGrid: '#3b82f6',
  Substation: '#8b5cf6',
  Hospital: '#ef4444',
  DataCenter: '#f59e0b',
  Generator: '#10b981',
  Road: '#6b7280',
};

export function InfraGraph({ highlightNodes = [] }: Props) {
  const [graphData, setGraphData] = useState<InfraGraph | null>(null);
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  useEffect(() => {
    getGraph()
      .then((data) => setGraphData(data))
      .catch(() => setGraphData(MOCK_GRAPH));
  }, []);

  useEffect(() => {
    if (!graphData) return;

    // Convert graph data to ReactFlow format
    const flowNodes: Node[] = graphData.nodes.map((n, idx) => {
      const isHighlighted = highlightNodes.includes(n.id);
      const color = NODE_TYPE_COLORS[n.type] || '#6b7280';
      
      return {
        id: n.id,
        type: 'default',
        position: {
          x: (idx % 4) * 250,
          y: Math.floor(idx / 4) * 150,
        },
        data: { label: n.label },
        style: {
          background: isHighlighted ? color : `${color}33`,
          border: `2px solid ${color}`,
          borderRadius: '8px',
          padding: '10px',
          color: '#fff',
          fontSize: '12px',
          fontWeight: 'bold',
          boxShadow: isHighlighted ? `0 0 20px ${color}` : 'none',
        },
      };
    });

    const flowEdges: Edge[] = graphData.edges.map((e, idx) => ({
      id: `e${idx}`,
      source: e.from,
      target: e.to,
      label: e.rel,
      animated: true,
      style: { stroke: '#00ffff', strokeWidth: 1 },
      labelStyle: { fill: '#8b949e', fontSize: 10 },
    }));

    setNodes(flowNodes);
    setEdges(flowEdges);
  }, [graphData, highlightNodes, setNodes, setEdges]);

  if (!graphData) {
    return (
      <div className="panel-glow h-full flex items-center justify-center">
        <div className="text-cyber-muted">Loading graph...</div>
      </div>
    );
  }

  return (
    <div className="panel-glow h-full">
      <div className="flex items-center gap-2 mb-3">
        <Network className="w-5 h-5 text-neon-cyan" />
        <h2 className="text-xl text-neon-cyan uppercase tracking-wider">
          Infrastructure Graph
        </h2>
      </div>
      <div style={{ height: 'calc(100% - 40px)' }} className="bg-cyber-darker rounded border border-cyber-border">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          fitView
          className="bg-cyber-darker"
        >
          <Background color="#1f2830" />
          <Controls />
        </ReactFlow>
      </div>
    </div>
  );
}
