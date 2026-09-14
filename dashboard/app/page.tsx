"use client";

import { fetchHealth, fetchStatus } from "@/lib/api";
import { usePoll } from "@/lib/use-poll";
import type { ComponentStatus } from "@/lib/types";

function Card({ row }: { row: ComponentStatus }) {
  return (
    <article className="card">
      <h3>
        <span className={`dot ${row.online ? "on" : "off"}`} />
        {row.id}
      </h3>
      <p className="muted">{row.kind}</p>
      <p className="mono">{row.last_commit ?? "no events yet"}</p>
    </article>
  );
}

export default function StatusPage() {
  const health = usePoll(fetchHealth, 4000);
  const status = usePoll(fetchStatus, 2000);

  return (
    <main>
      <p className="lede">
        Live view of the five kernel contexts and five RECON agents. The
        dashboard talks to the kernel operator API at{" "}
        <span className="mono">http://127.0.0.1:8080</span>.
      </p>
      {health.error || status.error ? (
        <div className="error">
          Kernel unreachable. Start it with{" "}
          <span className="mono">
            cargo run --manifest-path kernel/Cargo.toml
          </span>
          . {health.error || status.error}
        </div>
      ) : null}
      <div className="card" style={{ marginBottom: 16 }}>
        <h2>Kernel</h2>
        <p>
          <span
            className={`dot ${health.data?.status === "ok" ? "on" : "off"}`}
          />
          {health.data?.status === "ok" ? "online" : "offline"}
        </p>
      </div>
      <h2 className="muted">Contexts</h2>
      <div className="grid" style={{ marginBottom: 24 }}>
        {(status.data?.contexts ?? []).map((row) => (
          <Card key={row.id} row={row} />
        ))}
      </div>
      <h2 className="muted">Agents</h2>
      <div className="grid">
        {(status.data?.agents ?? []).map((row) => (
          <Card key={row.id} row={row} />
        ))}
      </div>
    </main>
  );
}
