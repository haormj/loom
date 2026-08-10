const BASE = '';

async function fetchJSON<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) throw new Error(`API ${path} failed: ${res.status}`);
  return res.json();
}

export interface ProjectStatus {
  initialized: boolean;
  activeDeliveryId: string | null;
  deliveries: Array<{ deliveryId: string; status: string; updatedAt: string }>;
}

export interface DeliverySummary {
  deliveryId: string;
  activePhaseId: string;
  status: string;
  phases: Array<{ phaseId: string; latestRefs: Record<string, string>; status: string }>;
  updatedAt: string;
}

export interface DeploySnapshot {
  prepared: boolean;
  state: unknown | null;
  logTail: string[];
  logRef: string | null;
  repairAction: unknown | null;
  failure: unknown | null;
}

export const api = {
  projectStatus: () => fetchJSON<ProjectStatus>('/api/project/status'),
  deliveries: () => fetchJSON<DeliverySummary[]>('/api/deliveries'),
  delivery: (id: string) => fetchJSON<DeliverySummary | null>(`/api/deliveries/${id}`),
  tasks: (deliveryId: string, phaseId: string) =>
    fetchJSON<unknown | null>(`/api/deliveries/${deliveryId}/phases/${phaseId}/tasks`),
  reviews: (deliveryId: string, phaseId: string) =>
    fetchJSON<unknown | null>(`/api/deliveries/${deliveryId}/phases/${phaseId}/reviews`),
  deployStatus: () => fetchJSON<DeploySnapshot>('/api/deploy/status'),
  knowledgeSources: () => fetchJSON<unknown[]>('/api/knowledge/sources'),
  auditRecords: () => fetchJSON<unknown>('/api/audit/records'),
};
