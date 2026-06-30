"use client";

function StatBlock({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </div>
      <div className="mt-2 text-2xl font-semibold text-foreground tabular-nums">
        {value}
      </div>
      {hint && (
        <div className="mt-1 text-xs text-muted-foreground">{hint}</div>
      )}
    </div>
  );
}

export function TotalMetrics() {
  return (
    <section className="grid grid-cols-1 gap-3 sm:grid-cols-3">
      <StatBlock label="Total Tokens" value="471.88M" />
      <StatBlock label="Total Requests" value="6.6K" />
      <StatBlock label="Peak Bucket" value="60.06M" />
    </section>
  );
}
