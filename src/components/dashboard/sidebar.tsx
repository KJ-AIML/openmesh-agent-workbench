"use client";

import {
  Home,
  Circle,
  BarChart3,
  Folder,
  Globe,
  Settings,
  ChevronRight,
  MessageSquare,
} from "lucide-react";
import { cn } from "@/lib/utils";

type SidebarItem = {
  label: string;
  icon?: React.ReactNode;
  active?: boolean;
  muted?: boolean;
};

function NavItem({ item }: { item: SidebarItem }) {
  return (
    <button
      type="button"
      className={cn(
        "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors",
        "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
        item.active
          ? "bg-sidebar-accent text-sidebar-accent-foreground"
          : "text-muted-foreground"
      )}
    >
      {item.icon && <span className="flex-shrink-0">{item.icon}</span>}
      <span className="truncate">{item.label}</span>
    </button>
  );
}

export function DashboardSidebar() {
  return (
    <aside className="hidden md:flex w-[220px] flex-shrink-0 flex-col border-r border-border bg-sidebar">
      {/* Brand */}
      <div className="flex h-14 items-center gap-2 px-4">
        <span className="text-base font-semibold text-foreground tracking-tight">
          OpenRouter
        </span>
      </div>

      {/* Nav */}
      <nav className="flex-1 overflow-y-auto px-2 py-2 space-y-4">
        <NavItem item={{ label: "Home", icon: <Home className="h-4 w-4" /> }} />

        {/* Workspace */}
        <div className="space-y-1">
          <div className="px-2 text-[11px] font-medium uppercase tracking-wider text-muted-foreground/70">
            Workspace
          </div>
          <NavItem
            item={{ label: "Status", icon: <Circle className="h-3.5 w-3.5" /> }}
          />
          <NavItem
            item={{
              label: "Usage",
              icon: <BarChart3 className="h-4 w-4" />,
              active: true,
            }}
          />
          <NavItem
            item={{ label: "Models", icon: <Folder className="h-4 w-4" /> }}
          />
          <NavItem
            item={{ label: "Server", icon: <Globe className="h-4 w-4" /> }}
          />
        </div>

        {/* Projects */}
        <div className="space-y-1">
          <div className="flex items-center justify-between px-2">
            <span className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground/70">
              Projects
            </span>
            <ChevronRight className="h-3 w-3 text-muted-foreground/60" />
          </div>
          {[
            "plugins",
            "personal",
            "vllm-studio",
            "deepseek-flas…",
            "parchi",
            "lambda",
            "ai",
          ].map((p) => (
            <NavItem
              key={p}
              item={{
                label: p,
                icon: <Folder className="h-3.5 w-3.5 opacity-60" />,
                muted: true,
              }}
            />
          ))}
        </div>

        {/* Chats */}
        <div className="space-y-1">
          <div className="flex items-center justify-between px-2">
            <span className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground/70">
              Chats
            </span>
            <ChevronRight className="h-3 w-3 text-muted-foreground/60" />
          </div>
          <NavItem
            item={{
              label: "No active chats",
              icon: <MessageSquare className="h-3.5 w-3.5 opacity-60" />,
              muted: true,
            }}
          />
        </div>
      </nav>

      {/* Settings */}
      <div className="border-t border-border p-2">
        <NavItem
          item={{ label: "Settings", icon: <Settings className="h-4 w-4" /> }}
        />
      </div>
    </aside>
  );
}
