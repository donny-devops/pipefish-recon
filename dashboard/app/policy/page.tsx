"use client";

import { fetchPolicy } from "@/lib/api";
import { usePoll } from "@/lib/use-poll";

export default function PolicyPage() {
  const { data, error } = usePoll(fetchPolicy, 8000);

  const allow = new Set((data?.allow ?? []).map(([a, t]) => `${a}|${t}`));
  const hil = new Set((data?.hil_gated ?? []).map(([a, t]) => `${a}|${t}`));

  return (
    <main>
      <p className="lede">
        Deny-by-default ACL loaded from <span className="mono">POLICY.md</span>{" "}
        §2. A1 cannot dispatch <span className="mono">nftables</span>. HIL-gated
        pairs are amber.
      </p>
      {error ? <div className="error">{error}</div> : null}
      {data ? (
        <div style={{ overflowX: "auto" }}>
          <table>
            <thead>
              <tr>
                <th>Tool</th>
                {data.agents.map((agent) => (
                  <th key={agent}>{agent}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {data.tools.map((tool) => (
                <tr key={tool}>
                  <th>{tool}</th>
                  {data.agents.map((agent) => {
                    const key = `${agent}|${tool}`;
                    const permitted = allow.has(key);
                    const gated = hil.has(key);
                    const cls = gated ? "hil" : permitted ? "allow" : "deny";
                    const label = gated ? "HIL" : permitted ? "allow" : "deny";
                    return (
                      <td key={key} className={cls}>
                        {label}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
    </main>
  );
}
