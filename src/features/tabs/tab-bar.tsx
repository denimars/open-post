import { Plus, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { useTabsStore } from "@/stores/tabs";

export function TabBar() {
  const { tabs, activeTabId, setActiveTab, closeTab, openTab } = useTabsStore();

  return (
    <div className="flex items-center gap-1 overflow-x-auto border-b border-border bg-muted/30 px-1">
      {tabs.map((tab) => (
        <div
          key={tab.id}
          role="tab"
          aria-selected={tab.id === activeTabId}
          tabIndex={0}
          onClick={() => setActiveTab(tab.id)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") setActiveTab(tab.id);
          }}
          className={cn(
            "group flex max-w-[16rem] shrink-0 cursor-pointer items-center gap-2 border-b-2 px-3 py-2 text-xs",
            tab.id === activeTabId
              ? "border-primary bg-background text-foreground"
              : "border-transparent text-muted-foreground hover:text-foreground",
          )}
        >
          <span className="truncate">{tab.title}</span>
          {tab.isDirty && <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-amber-500" />}
          <button
            aria-label={`Close ${tab.title}`}
            onClick={(e) => {
              e.stopPropagation();
              closeTab(tab.id);
            }}
            className="rounded p-0.5 opacity-0 transition-opacity hover:bg-muted group-hover:opacity-100"
          >
            <X className="h-3 w-3" />
          </button>
        </div>
      ))}

      <button
        aria-label="New request"
        onClick={() => openTab()}
        className="shrink-0 rounded p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
      >
        <Plus className="h-4 w-4" />
      </button>
    </div>
  );
}
