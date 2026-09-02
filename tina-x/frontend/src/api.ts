import type { InfraGraph, Scenario, SimulationResult, Alert } from './types';

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8080';

async function fetchJSON<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, init);
  if (!res.ok) {
    throw new Error(`API error: ${res.status} ${res.statusText}`);
  }
  return res.json();
}

export async function getHealth(): Promise<{ status: string }> {
  return fetchJSON('/api/health');
}

export async function getScenarios(): Promise<{ scenarios: Scenario[] }> {
  return fetchJSON('/api/scenarios');
}

export async function getGraph(): Promise<InfraGraph> {
  return fetchJSON('/api/graph');
}

export async function simulate(scenario: string): Promise<SimulationResult> {
  return fetchJSON('/api/simulate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ scenario }),
  });
}

export async function simulateCustom(changes: Record<string, string>): Promise<SimulationResult> {
  return fetchJSON('/api/simulate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ changes }),
  });
}

export async function getAlerts(): Promise<{ alerts: Alert[] }> {
  return fetchJSON('/api/alerts');
}

// Mock data for when API is unavailable (graceful degradation for Vercel static deploy)
export const MOCK_GRAPH: InfraGraph = {
  nodes: [
    { id: 'grid-A', type: 'PowerGrid', label: 'Grid A' },
    { id: 'grid-B', type: 'PowerGrid', label: 'Grid B' },
    { id: 'substation-X', type: 'Substation', label: 'Substation X' },
    { id: 'hospital-B', type: 'Hospital', label: 'Hospital B' },
    { id: 'hospital-C', type: 'Hospital', label: 'Hospital C' },
    { id: 'generator-1', type: 'Generator', label: 'Generator 1' },
    { id: 'generator-2', type: 'Generator', label: 'Generator 2' },
    { id: 'road-3', type: 'Road', label: 'Road 3' },
    { id: 'road-5', type: 'Road', label: 'Road 5' },
    { id: 'datacenter-1', type: 'DataCenter', label: 'Data Center 1' },
  ],
  edges: [
    { from: 'substation-X', to: 'grid-A', rel: 'feeds' },
    { from: 'grid-A', to: 'hospital-B', rel: 'powers' },
    { from: 'grid-A', to: 'datacenter-1', rel: 'powers' },
    { from: 'grid-B', to: 'hospital-C', rel: 'powers' },
    { from: 'hospital-B', to: 'generator-1', rel: 'has-backup' },
    { from: 'hospital-C', to: 'generator-2', rel: 'has-backup' },
    { from: 'road-3', to: 'generator-1', rel: 'fuel-via' },
    { from: 'road-5', to: 'generator-2', rel: 'fuel-via' },
    { from: 'road-3', to: 'hospital-B', rel: 'road-access' },
    { from: 'road-5', to: 'hospital-C', rel: 'road-access' },
  ],
};

export const MOCK_SCENARIOS: Scenario[] = [
  {
    key: 'quake-typhoon',
    name: 'Compound: Earthquake + Typhoon',
    description: 'Earthquake damages grid, typhoon floods roads — cascade to hospital failure',
    changes: { 'substation-X': 'offline', 'road-3': 'flooded' },
  },
  {
    key: 'cyber-physical',
    name: 'Cyber-Physical Cascade',
    description: 'Substation offline + datacenter attack → cascading failure',
    changes: { 'substation-X': 'offline', 'datacenter-1': 'compromised' },
  },
];
