export type ComponentStatus = {
  id: string;
  kind: "agent" | "context";
  online: boolean;
  last_event_at: string | null;
  last_commit: string | null;
};

export type StatusResponse = {
  kernel: string;
  contexts: ComponentStatus[];
  agents: ComponentStatus[];
};

export type BusEvent = {
  id: string;
  timestamp: string;
  source: string;
  commit_type: string;
  scope: string;
  description: string;
  severity: string | null;
  payload: unknown;
};

export type PolicyMatrix = {
  agents: string[];
  tools: string[];
  allow: [string, string][];
  hil_gated: [string, string][];
};

export type HealthResponse = {
  status: string;
};
