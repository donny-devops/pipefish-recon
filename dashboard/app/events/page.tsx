"use client";

import { FormEvent, useState } from "react";
import { fetchRecentEvents, ingestSignal } from "@/lib/api";
import { usePoll } from "@/lib/use-poll";

export default function EventsPage() {
  const { data, error } = usePoll(fetchRecentEvents, 2000);
  const [cve, setCve] = useState("CVE-2026-1337");
  const [cvss, setCvss] = useState("9.8");
  const [summary, setSummary] = useState("operator-ingested test signal");
  const [ingestMsg, setIngestMsg] = useState<string | null>(null);

  async function onIngest(e: FormEvent) {
    e.preventDefault();
    setIngestMsg(null);
    try {
      const score = Number.parseFloat(cvss);
      await ingestSignal({
        cve_id: cve,
        cvss: Number.isFinite(score) ? score : undefined,
        summary,
      });
      setIngestMsg(`accepted ${cve}`);
    } catch (err) {
      setIngestMsg(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <main>
      <p className="lede">
        Conventional Commit bus stream. Ingest a webhook signal to watch
        RECON-A1 normalize it and RECON-A3 route by CVSS.
      </p>
      {error ? <div className="error">{error}</div> : null}
      <form className="card ingest" onSubmit={onIngest}>
        <h2>Webhook ingest</h2>
        <label>
          CVE
          <input value={cve} onChange={(e) => setCve(e.target.value)} required />
        </label>
        <label>
          CVSS
          <input value={cvss} onChange={(e) => setCvss(e.target.value)} />
        </label>
        <label>
          Summary
          <input value={summary} onChange={(e) => setSummary(e.target.value)} />
        </label>
        <button type="submit">Ingest</button>
        {ingestMsg ? <p className="muted">{ingestMsg}</p> : null}
      </form>
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
              {evt.severity ? (
                <span className={`pill sev-${evt.severity}`}>{evt.severity}</span>
              ) : (
                <span />
              )}
              <span className="mono">
                {evt.commit_type}({evt.scope}): {evt.description}
              </span>
            </div>
          ))}
      </section>
    </main>
  );
}
