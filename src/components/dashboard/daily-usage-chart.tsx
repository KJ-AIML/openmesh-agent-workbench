"use client";

import {
  ResponsiveContainer,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
} from "recharts";
import { useMemo, useState } from "react";
import { MODELS, MODEL_COLORS, buildDailyUsage } from "@/lib/dashboard-data";

const data = buildDailyUsage();

function CustomTooltip({ active, payload }: any) {
  if (!active || !payload || !payload.length) return null;
  const p = payload[0]?.payload;
  if (!p) return null;
  return (
    <div className="rounded-md border border-border bg-popover px-3 py-2 text-xs shadow-lg">
      <div className="font-medium text-foreground">
        {p.label} · {p.total} REQ
      </div>
      <div className="mt-0.5 text-muted-foreground tabular-nums">
        {p.totalTokens.toLocaleString()}
      </div>
      <div className="mt-2 space-y-1">
        {MODELS.filter((m) => p.values[m] > 0).map((m) => {
          const pct = ((p.values[m] / p.total) * 100).toFixed(1);
          return (
            <div key={m} className="flex items-center gap-2">
              <span
                className="h-2 w-2 rounded-sm"
                style={{ background: MODEL_COLORS[m] }}
              />
              <span className="text-foreground">{m}</span>
              <span className="ml-auto text-muted-foreground tabular-nums">
                {p.values[m].toLocaleString()} · {pct}%
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function DailyUsageChart() {
  const [hidden, setHidden] = useState<Set<string>>(new Set());

  const chartData = useMemo(
    () =>
      data.map((d) => ({
        label: d.label,
        short: d.short,
        total: d.total,
        totalTokens: d.totalTokens,
        ...d.values,
      })),
    []
  );

  const toggle = (m: string) => {
    setHidden((prev) => {
      const next = new Set(prev);
      if (next.has(m)) next.delete(m);
      else next.add(m);
      return next;
    });
  };

  return (
    <section className="rounded-lg border border-border bg-card p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            Daily Usage
          </div>
          <div className="mt-0.5 text-sm text-foreground">
            Requests per day, stacked by model
          </div>
        </div>
      </div>

      <div className="mt-4 h-64 w-full">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart
            data={chartData}
            margin={{ top: 8, right: 8, left: -16, bottom: 0 }}
            barCategoryGap="18%"
          >
            <CartesianGrid
              strokeDasharray="3 3"
              stroke="rgba(255,255,255,0.05)"
              vertical={false}
            />
            <XAxis
              dataKey="short"
              tick={{ fill: "#888", fontSize: 11 }}
              tickLine={false}
              axisLine={{ stroke: "rgba(255,255,255,0.1)" }}
              interval={3}
            />
            <YAxis
              tick={{ fill: "#888", fontSize: 11 }}
              tickLine={false}
              axisLine={false}
              width={48}
              tickFormatter={(v) => `${v}`}
            />
            <Tooltip
              cursor={{ fill: "rgba(255,255,255,0.04)" }}
              content={<CustomTooltip />}
            />
            {MODELS.map((m) => (
              <Bar
                key={m}
                dataKey={m}
                stackId="a"
                fill={MODEL_COLORS[m]}
                hide={hidden.has(m)}
                radius={[0, 0, 0, 0]}
              />
            ))}
          </BarChart>
        </ResponsiveContainer>
      </div>

      {/* Legend */}
      <div className="mt-3 flex flex-wrap gap-x-4 gap-y-2">
        {MODELS.map((m) => {
          const isHidden = hidden.has(m);
          return (
            <button
              key={m}
              type="button"
              onClick={() => toggle(m)}
              className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
              style={{ opacity: isHidden ? 0.4 : 1 }}
            >
              <span
                className="h-2.5 w-2.5 rounded-sm"
                style={{ background: MODEL_COLORS[m] }}
              />
              {m}
            </button>
          );
        })}
      </div>
    </section>
  );
}
