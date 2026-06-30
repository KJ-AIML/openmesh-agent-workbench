"use client";

import { RefreshCw } from "lucide-react";

function MetricCard({
  label,
  value,
  sub,
}: {
  label: string;
  value: string;
  sub?: string;
}) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </div>
      <div className="mt-2 text-xl font-semibold text-foreground tabular-nums">
        {value}
      </div>
      {sub && (
        <div className="mt-1 text-xs text-muted-foreground tabular-nums">
          {sub}
        </div>
      )}
    </div>
  );
}

export function UsageSummary() {
  return (
    <section>
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            USAGE controller
          </div>
          <h1 className="mt-1 text-3xl font-semibold tracking-tight text-foreground tabular-nums">
            623.48M tokens
          </h1>
          <p className="mt-1 text-sm text-muted-foreground tabular-nums">
            9.1K requests · 11 sessions · 0 users
          </p>
        </div>
      </div>

      <div className="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
        <MetricCard
          label="Prompt"
          value="618.71M"
          sub="input tokens"
        />
        <MetricCard
          label="Completion"
          value="4.77M"
          sub="output tokens"
        />
        <MetricCard
          label="24H REQ"
          value="495"
          sub="112 last hour"
        />
        <MetricCard
          label="Avg Tokens"
          value="68.8K"
          sub="68.3K in · 527 out"
        />
        <div className="col-span-2 lg:col-span-4 flex justify-end">
          <button
            type="button"
            aria-label="Refresh"
            className="flex items-center gap-2 rounded-md border border-border bg-card px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground hover:bg-sidebar-accent transition-colors"
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Refresh
          </button>
        </div>
      </div>
    </section>
  );
}
