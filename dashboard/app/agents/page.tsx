"use client";

import { AGENT_ROLES } from "@/lib/agents";
import { fetchStatus } from "@/lib/api";
import { usePoll } from "@/lib/use-poll";

export default function AgentsPage() {
  const { data, error } = usePoll(fetchStatus, 2000);
  const online = new Map((data?.agents ?? []).map((a) => [a.id, a]));

  return (
    <main>
      <p className="lede">
        Role definitions from <span className="mono">SOUL.md</span>. Online
        flags come from the kernel bus, not from live tool dispatch.
      </p>
      {error ? <div className="error">{error}</div> : null}
      <div className="grid">
        {Object.entries(AGENT_ROLES).map(([id, role]) => {
          const row = online.get(id);
          return (
            <article className="card" key={id}>
              <h3>
                <span className={`dot ${row?.online ? "on" : "off"}`} />
                {role.name}
              </h3>
              <p className="muted">commit scope: {role.scope}</p>
              <p>{role.mission}</p>
              <p className="mono muted">{row?.last_commit ?? "offline"}</p>
            </article>
          );
        })}
      </div>
    </main>
  );
}
