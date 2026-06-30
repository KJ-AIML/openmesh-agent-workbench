"use client";

import { DashboardSidebar } from "@/components/dashboard/sidebar";
import { UsageSummary } from "@/components/dashboard/usage-summary";
import { DailyUsageChart } from "@/components/dashboard/daily-usage-chart";
import { TotalMetrics } from "@/components/dashboard/total-metrics";
import { ModelPerformanceTable } from "@/components/dashboard/model-performance-table";
import { cn } from "@/lib/utils";
import { useState } from "react";

const TOP_NAV = ["SOURCE", "Provider", "Pi sessions"];

export default function Home() {
  const [activeTab, setActiveTab] = useState("SOURCE");

  return (
    <div className="flex min-h-screen w-full bg-background text-foreground">
      <DashboardSidebar />

      {/* Main content */}
      <div className="flex flex-1 flex-col min-w-0">
        {/* Top mini nav */}
        <header className="sticky top-0 z-10 flex h-14 items-center gap-1 border-b border-border bg-background/80 px-4 backdrop-blur">
          <nav className="flex items-center gap-1 text-sm">
            {TOP_NAV.map((t, idx) => (
              <span key={t} className="flex items-center">
                <button
                  type="button"
                  onClick={() => setActiveTab(t)}
                  className={cn(
                    "rounded-md px-2 py-1 transition-colors",
                    activeTab === t
                      ? "text-foreground"
                      : "text-muted-foreground hover:text-foreground"
                  )}
                >
                  {t}
                </button>
                {idx < TOP_NAV.length - 1 && (
                  <span className="px-1 text-muted-foreground/40">/</span>
                )}
              </span>
            ))}
          </nav>
        </header>

        <main className="flex-1 space-y-6 p-4 md:p-6 lg:p-8">
          <UsageSummary />
          <DailyUsageChart />
          <TotalMetrics />
          <ModelPerformanceTable />
        </main>
      </div>
    </div>
  );
}
