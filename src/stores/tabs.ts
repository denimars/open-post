import { create } from "zustand";
import { createId } from "@/lib/utils";
import { mergeParams, paramsFromUrl, urlWithParams } from "@/lib/url";
import type { AppError, HttpRequest, HttpResponse, KeyValue } from "@/types";

/** Which entry in a collection this tab is linked to. */
export interface SavedTo {
  collectionId: string;
  /** Id of the row in `saved_requests` — never the request's own id. */
  savedRequestId: string;
  name: string;
}

export interface Tab {
  id: string;
  title: string;
  request: HttpRequest;
  response: HttpResponse | null;
  error: AppError | null;
  isSending: boolean;
  /** Set once the tab has been saved into a collection. */
  savedTo: SavedTo | null;
  isDirty: boolean;
}

export function emptyRequest(): HttpRequest {
  return {
    id: createId(),
    method: "GET",
    url: "",
    params: [],
    headers: [],
    body: { type: "none" },
    auth: { type: "none" },
    timeoutMs: 30_000,
    name: null,
  };
}

export function newTab(request?: HttpRequest, title?: string, savedTo?: SavedTo | null): Tab {
  const req = request ?? emptyRequest();
  return {
    id: createId(),
    title: title ?? "Untitled request",
    request: req,
    response: null,
    error: null,
    isSending: false,
    savedTo: savedTo ?? null,
    isDirty: false,
  };
}

interface TabsState {
  tabs: Tab[];
  activeTabId: string | null;
  activeEnvironmentId: string | null;

  openTab: (request?: HttpRequest, title?: string, savedTo?: SavedTo | null) => string;
  /**
   * Opens a request stored in a collection. If a tab for that same saved entry
   * is already open it is focused instead of opening a duplicate.
   */
  openSavedRequest: (request: HttpRequest, savedTo: SavedTo) => string;
  closeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  /** Patches the active tab's request and keeps URL and params in sync. */
  updateRequest: (id: string, patch: Partial<HttpRequest>) => void;
  /** URL typed by the user: re-derive the params table from it. */
  setUrl: (id: string, url: string) => void;
  /** Params table edited: rewrite the URL's query string. */
  setParams: (id: string, params: KeyValue[]) => void;
  setTabState: (id: string, patch: Partial<Omit<Tab, "id">>) => void;
  setActiveEnvironment: (id: string | null) => void;
}

function replace(tabs: Tab[], id: string, update: (tab: Tab) => Tab): Tab[] {
  return tabs.map((tab) => (tab.id === id ? update(tab) : tab));
}

/** A readable tab label derived from the URL. */
export function titleFor(request: HttpRequest): string {
  if (request.name) return request.name;
  const url = request.url.trim();
  if (!url) return "Untitled request";
  const withoutQuery = url.split("?")[0] ?? url;
  const path = withoutQuery.replace(/^[a-z]+:\/\//i, "");
  return path.length > 40 ? `${path.slice(0, 39)}…` : path;
}

export const useTabsStore = create<TabsState>((set, get) => ({
  tabs: [],
  activeTabId: null,
  activeEnvironmentId: null,

  openTab: (request, title, savedTo) => {
    const tab = newTab(request, title, savedTo);
    set((state) => ({ tabs: [...state.tabs, tab], activeTabId: tab.id }));
    return tab.id;
  },

  openSavedRequest: (request, savedTo) => {
    const existing = get().tabs.find(
      (tab) => tab.savedTo?.savedRequestId === savedTo.savedRequestId,
    );
    if (existing) {
      set({ activeTabId: existing.id });
      return existing.id;
    }
    const tab = newTab(request, savedTo.name, savedTo);
    set((state) => ({ tabs: [...state.tabs, tab], activeTabId: tab.id }));
    return tab.id;
  },

  closeTab: (id) =>
    set((state) => {
      const index = state.tabs.findIndex((tab) => tab.id === id);
      if (index === -1) return state;
      const tabs = state.tabs.filter((tab) => tab.id !== id);
      let activeTabId = state.activeTabId;
      if (state.activeTabId === id) {
        const neighbour = tabs[index] ?? tabs[index - 1] ?? null;
        activeTabId = neighbour?.id ?? null;
      }
      return { tabs, activeTabId };
    }),

  setActiveTab: (id) => set({ activeTabId: id }),

  updateRequest: (id, patch) =>
    set((state) => ({
      tabs: replace(state.tabs, id, (tab) => {
        const request = { ...tab.request, ...patch };
        return { ...tab, request, isDirty: true, title: titleFor(request) };
      }),
    })),

  setUrl: (id, url) =>
    set((state) => ({
      tabs: replace(state.tabs, id, (tab) => {
        const request = {
          ...tab.request,
          url,
          params: mergeParams(tab.request.params, paramsFromUrl(url)),
        };
        return { ...tab, request, isDirty: true, title: titleFor(request) };
      }),
    })),

  setParams: (id, params) =>
    set((state) => ({
      tabs: replace(state.tabs, id, (tab) => {
        const request = {
          ...tab.request,
          params,
          url: urlWithParams(tab.request.url, params),
        };
        return { ...tab, request, isDirty: true, title: titleFor(request) };
      }),
    })),

  setTabState: (id, patch) =>
    set((state) => ({ tabs: replace(state.tabs, id, (tab) => ({ ...tab, ...patch })) })),

  setActiveEnvironment: (id) => set({ activeEnvironmentId: id }),
}));

export function activeTab(): Tab | null {
  const { tabs, activeTabId } = useTabsStore.getState();
  return tabs.find((tab) => tab.id === activeTabId) ?? null;
}
