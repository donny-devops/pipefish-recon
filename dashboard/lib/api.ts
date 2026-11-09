import type {
  HealthResponse,
  PolicyMatrix,
  StatusResponse,
  BusEvent,
} from "./types";

export const KERNEL_URL =
  process.env.NEXT_PUBLIC_KERNEL_URL ?? "http://127.0.0.1:8080";

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${KERNEL_URL}${path}`, { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`${path} failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as T;
}

export function fetchHealth(): Promise<HealthResponse> {
  return getJson("/health");
}

export function fetchStatus(): Promise<StatusResponse> {
  return getJson("/api/status");
}

export function fetchRecentEvents(): Promise<BusEvent[]> {
  return getJson("/api/events?limit=80");
}

export function fetchPolicy(): Promise<PolicyMatrix> {
  return getJson("/api/policy");
}

export async function ingestSignal(body: {
  cve_id: string;
  cvss?: number;
  summary?: string;
}): Promise<void> {
  const res = await fetch(`${KERNEL_URL}/api/ingest`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(`ingest failed: ${res.status} ${res.statusText}`);
  }
}
