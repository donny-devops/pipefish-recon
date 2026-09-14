"use client";

import { fetchRecentEvents } from "@/lib/api";
import { usePoll } from "@/lib/use-poll";

export default function EventsPage() {
  const { data, error } = usePoll(fetchRecentEvents, 2000);

  return (
    <main>
      <p className="lede">
        Conventional Commit bus stream. Every kernel action is one row in{" "}
        <span className="mono">audit_log.jsonl</span> and here.
      </p>
      {error ? <div className="error">{error}</div> : null}
      <section className="card">
        {(data ?? []).length === 0 && !error ? (
          <p className="muted">Waiting for bus events…</p>
        ) : null}
        {(data ?? [])
          .slice()
          .reverse()
          .map((evt) => (
            <div className="event-row" key={evt.id}>
              <time className="mono">{evt.timestamp}</time>
              <span className="pill">{evt.source}</span>
              <span className="mono">
                {evt.commit_type}({evt.scope}): {evt.description}
              </span>
            </div>
          ))}
      </section>
    </main>
  );
}
