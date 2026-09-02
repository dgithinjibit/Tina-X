export interface GraphNode {
  id: string;
  type: string;
  label: string;
}

export interface GraphEdge {
  from: string;
  to: string;
  rel: string;
}

export interface InfraGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface Scenario {
  key: string;
  name: string;
  description: string;
  changes: Record<string, string>;
}

export interface Failure {
  kind: string;
  node: string;
  message: string;
}

export interface SimulationResult {
  scenario: Scenario;
  failures: Failure[];
  statuses: Record<string, string>;
  failure_count: number;
}

export interface Alert {
  level: 'info' | 'warning' | 'critical';
  message: string;
  timestamp?: string;
}
