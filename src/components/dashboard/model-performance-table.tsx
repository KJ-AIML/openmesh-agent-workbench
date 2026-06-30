"use client";

import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import { useState } from "react";
import { MODEL_COLORS, MODEL_PERFORMANCE, type ModelPerf } from "@/lib/dashboard-data";
import { cn } from "@/lib/utils";

type SortKey = "requests" | "tokens" | "latency" | "ttft" | "speed";

function SortIcon({
  k,
  sortKey,
  asc,
}: {
  k: SortKey;
  sortKey: SortKey;
  asc: boolean;
}) {
  if (k !== sortKey)
    return <ArrowUpDown className="h-3 w-3 opacity-40" />;
  return asc ? (
    <ArrowUp className="h-3 w-3 text-foreground" />
  ) : (
    <ArrowDown className="h-3 w-3 text-foreground" />
  );
}

function formatSpeed(m: ModelPerf) {
  const parts: string[] = [];
  if (m.prefill > 0) parts.push(`${Math.round(m.prefill / 1000)}K prefill/s`);
  if (m.gen > 0) parts.push(`${m.gen} gen/s`);
  return parts.join(" · ");
}

function ProgressBar({
  value,
  max,
  color,
  display,
  align = "left",
}: {
  value: number;
  max: number;
  color: string;
  display: string;
  align?: "left" | "right";
}) {
  const pct = max > 0 ? Math.max(2, Math.min(100, (value / max) * 100)) : 0;
  return (
    <div className="flex items-center gap-2">
      <div
        className={cn(
          "relative h-1.5 w-20 overflow-hidden rounded-full bg-muted",
          align === "right" && "order-2"
        )}
      >
        <div
          className="absolute inset-y-0 left-0 rounded-full"
          style={{ width: `${pct}%`, background: color }}
        />
      </div>
      <span
        className={cn(
          "text-xs tabular-nums text-foreground",
          align === "right" ? "order-1 text-right" : "order-2"
        )}
      >
        {display}
      </span>
    </div>
  );
}

export function ModelPerformanceTable() {
  const [sortKey, setSortKey] = useState<SortKey>("tokens");
  const [asc, setAsc] = useState(false);

  const sorted = [...MODEL_PERFORMANCE].sort((a, b) => {
    let av: number, bv: number;
    switch (sortKey) {
      case "requests":
        av = a.requests;
        bv = b.requests;
        break;
      case "tokens":
        av = a.tokens;
        bv = b.tokens;
        break;
      case "latency":
        av = a.latency;
        bv = b.latency;
        break;
      case "ttft":
        av = a.ttft;
        bv = b.ttft;
        break;
      case "speed":
        av = a.prefill + a.gen;
        bv = b.prefill + b.gen;
        break;
    }
    return asc ? av - bv : bv - av;
  });

  const maxReq = Math.max(...MODEL_PERFORMANCE.map((m) => m.requests));
  const maxTok = Math.max(...MODEL_PERFORMANCE.map((m) => m.tokens));
  const maxLat = Math.max(...MODEL_PERFORMANCE.map((m) => m.latency));
  const maxTtft = Math.max(...MODEL_PERFORMANCE.map((m) => m.ttft));
  const maxSpeed = Math.max(
    ...MODEL_PERFORMANCE.map((m) => m.prefill + m.gen)
  );

  const toggleSort = (k: SortKey) => {
    if (k === sortKey) setAsc(!asc);
    else {
      setSortKey(k);
      setAsc(false);
    }
  };

  return (
    <section className="rounded-lg border border-border bg-card">
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
          Model Performance
        </div>
        <div className="text-xs text-muted-foreground">
          {MODEL_PERFORMANCE.length} models
        </div>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full min-w-[760px] text-sm">
          <thead>
            <tr className="text-[11px] uppercase tracking-wider text-muted-foreground">
              <th className="px-4 py-2.5 text-left font-medium">Model</th>
              <th className="px-4 py-2.5 text-left font-medium">
                <button
                  type="button"
                  className="flex items-center gap-1 hover:text-foreground"
                  onClick={() => toggleSort("requests")}
                >
                  Requests <SortIcon k="requests" sortKey={sortKey} asc={asc} />
                </button>
              </th>
              <th className="px-4 py-2.5 text-left font-medium">
                <button
                  type="button"
                  className="flex items-center gap-1 hover:text-foreground"
                  onClick={() => toggleSort("tokens")}
                >
                  Tokens <SortIcon k="tokens" sortKey={sortKey} asc={asc} />
                </button>
              </th>
              <th className="px-4 py-2.5 text-left font-medium">
                <button
                  type="button"
                  className="flex items-center gap-1 hover:text-foreground"
                  onClick={() => toggleSort("latency")}
                >
                  Latency <SortIcon k="latency" sortKey={sortKey} asc={asc} />
                </button>
              </th>
              <th className="px-4 py-2.5 text-left font-medium">
                <button
                  type="button"
                  className="flex items-center gap-1 hover:text-foreground"
                  onClick={() => toggleSort("ttft")}
                >
                  TTFT <SortIcon k="ttft" sortKey={sortKey} asc={asc} />
                </button>
              </th>
              <th className="px-4 py-2.5 text-left font-medium">
                <button
                  type="button"
                  className="flex items-center gap-1 hover:text-foreground"
                  onClick={() => toggleSort("speed")}
                >
                  Speed <SortIcon k="speed" sortKey={sortKey} asc={asc} />
                </button>
              </th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((m) => (
              <tr
                key={m.model}
                className="border-t border-border/60 hover:bg-sidebar-accent/40 transition-colors"
              >
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <span
                      className="h-2.5 w-2.5 rounded-sm"
                      style={{ background: MODEL_COLORS[m.model] }}
                    />
                    <span className="font-medium text-foreground">
                      {m.model}
                    </span>
                  </div>
                </td>
                <td className="px-4 py-3">
                  <ProgressBar
                    value={m.requests}
                    max={maxReq}
                    color="#50C878"
                    display={m.requestsLabel}
                  />
                </td>
                <td className="px-4 py-3">
                  <ProgressBar
                    value={m.tokens}
                    max={maxTok}
                    color="#50C878"
                    display={m.tokensLabel}
                  />
                </td>
                <td className="px-4 py-3">
                  <ProgressBar
                    value={m.latency}
                    max={maxLat}
                    color="#FF6B6B"
                    display={`${m.latency.toFixed(1)}s`}
                  />
                </td>
                <td className="px-4 py-3">
                  <ProgressBar
                    value={m.ttft}
                    max={maxTtft}
                    color="#FF6B6B"
                    display={`${m.ttft.toFixed(1)}s`}
                  />
                </td>
                <td className="px-4 py-3">
                  <ProgressBar
                    value={m.prefill + m.gen}
                    max={maxSpeed}
                    color="#50C878"
                    display={formatSpeed(m)}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
