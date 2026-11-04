import { useCallback, useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Moon, Sun } from "lucide-react";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { Sidebar } from "./sidebar";
import { useTheme } from "./theme-context";
import { TabBar } from "@/features/tabs/tab-bar";
import { RequestPanel } from "@/features/request/request-panel";
import { ResponsePanel } from "@/features/response/response-panel";
import { EnvironmentPicker } from "@/features/environments/environment-picker";
import { EnvironmentManager } from "@/features/environments/environment-manager";
import { SaveRequestDialog } from "@/features/collections/save-request-dialog";
import { cancelRequest, sendRequest } from "@/lib/tauri";
import diyaLogo from "@/assets/diya-logo.png";
import { toAppError } from "@/types";
import { useTabsStore } from "@/stores/tabs";

export function App() {
  const queryClient = useQueryClient();
  const { resolved, setMode } = useTheme();
  const [showEnvironments, setShowEnvironments] = useState(false);
  const [showSave, setShowSave] = useState(false);

  const { tabs, activeTabId, openTab, setTabState, activeEnvironmentId } = useTabsStore();
  const tab = tabs.find((candidate) => candidate.id === activeTabId) ?? null;

  // Always keep one tab open.
  useEffect(() => {
    if (tabs.length === 0) openTab();
  }, [tabs.length, openTab]);

  const send = useCallback(async () => {
    if (!tab || tab.isSending || tab.request.url.trim() === "") return;
    const tabId = tab.id;
    setTabState(tabId, { isSending: true, error: null });
    try {
      const response = await sendRequest(tab.request, activeEnvironmentId);
      setTabState(tabId, { response, error: null, isSending: false });
    } catch (error) {
      setTabState(tabId, { error: toAppError(error), response: null, isSending: false });
    } finally {
      queryClient.invalidateQueries({ queryKey: ["history"] });
    }
  }, [tab, activeEnvironmentId, setTabState, queryClient]);

  const cancel = useCallback(async () => {
    if (!tab) return;
    await cancelRequest(tab.request.id).catch(() => false);
  }, [tab]);

  // Ctrl/Cmd+Enter sends, Ctrl/Cmd+S saves.
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!(event.metaKey || event.ctrlKey)) return;
      if (event.key === "Enter") {
        event.preventDefault();
        void send();
      } else if (event.key.toLowerCase() === "s") {
        event.preventDefault();
        setShowSave(true);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [send]);

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <header className="flex items-center justify-between border-b border-border px-3 py-1.5">
        <div className="flex items-center gap-2">
          {/* The wordmark is a single navy colour, so it is inverted to white in dark mode. */}
          <img
            src={diyaLogo}
            alt="diya PROJECT"
            className="h-5 w-auto dark:brightness-0 dark:invert"
          />
          <span className="text-sm font-semibold">Open Post</span>
        </div>
        <div className="flex items-center gap-2">
          <EnvironmentPicker onManage={() => setShowEnvironments(true)} />
          <Button
            variant="ghost"
            size="icon"
            aria-label="Toggle theme"
            onClick={() => setMode(resolved === "dark" ? "light" : "dark")}
          >
            {resolved === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
          </Button>
        </div>
      </header>

      <PanelGroup direction="horizontal" className="min-h-0 flex-1">
        <Panel defaultSize={22} minSize={14} maxSize={40}>
          <Sidebar />
        </Panel>
        <PanelResizeHandle className="w-px bg-border transition-colors hover:bg-primary" />

        <Panel>
          <div className="flex h-full flex-col">
            <TabBar />
            {tab ? (
              <PanelGroup direction="vertical" className="min-h-0 flex-1">
                <Panel defaultSize={50} minSize={20}>
                  <RequestPanel
                    tab={tab}
                    onSend={() => void send()}
                    onCancel={() => void cancel()}
                    onSave={() => setShowSave(true)}
                  />
                </Panel>
                <PanelResizeHandle className="h-px bg-border transition-colors hover:bg-primary" />
                <Panel defaultSize={50} minSize={20}>
                  <ResponsePanel
                    response={tab.response}
                    error={tab.error}
                    isSending={tab.isSending}
                  />
                </Panel>
              </PanelGroup>
            ) : (
              <EmptyState title="No request open" hint="Press + to start a new request." />
            )}
          </div>
        </Panel>
      </PanelGroup>

      <EnvironmentManager
        open={showEnvironments}
        onClose={() => setShowEnvironments(false)}
      />

      <SaveRequestDialog
        open={showSave}
        request={tab?.request ?? null}
        defaultName={tab?.savedTo?.name ?? tab?.title ?? "New request"}
        onClose={() => setShowSave(false)}
        onSaved={(collectionId, name) => {
          if (tab) setTabState(tab.id, { savedTo: { collectionId, name }, isDirty: false, title: name });
        }}
      />
    </div>
  );
}
