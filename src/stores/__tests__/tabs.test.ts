import { beforeEach, describe, expect, it } from "vitest";
import { emptyRequest, titleFor, useTabsStore } from "@/stores/tabs";

function reset() {
  useTabsStore.setState({ tabs: [], activeTabId: null, activeEnvironmentId: null });
}

describe("tabs store", () => {
  beforeEach(reset);

  it("opens a tab and makes it active", () => {
    const id = useTabsStore.getState().openTab();
    const state = useTabsStore.getState();
    expect(state.tabs).toHaveLength(1);
    expect(state.activeTabId).toBe(id);
    expect(state.tabs[0]?.title).toBe("Untitled request");
  });

  it("gives each tab its own request id", () => {
    const { openTab } = useTabsStore.getState();
    openTab();
    openTab();
    const [first, second] = useTabsStore.getState().tabs;
    expect(first?.request.id).not.toBe(second?.request.id);
  });

  it("activates the next tab when the active one is closed", () => {
    const { openTab } = useTabsStore.getState();
    const first = openTab();
    const second = openTab();
    useTabsStore.getState().setActiveTab(first);
    useTabsStore.getState().closeTab(first);
    expect(useTabsStore.getState().activeTabId).toBe(second);
  });

  it("falls back to the previous tab when the last one is closed", () => {
    const { openTab } = useTabsStore.getState();
    const first = openTab();
    const second = openTab();
    useTabsStore.getState().closeTab(second);
    expect(useTabsStore.getState().activeTabId).toBe(first);
  });

  it("clears the active tab when the last tab closes", () => {
    const id = useTabsStore.getState().openTab();
    useTabsStore.getState().closeTab(id);
    expect(useTabsStore.getState().tabs).toHaveLength(0);
    expect(useTabsStore.getState().activeTabId).toBeNull();
  });

  it("derives the params table from a typed URL", () => {
    const id = useTabsStore.getState().openTab();
    useTabsStore.getState().setUrl(id, "https://a.test/users?page=2");
    const tab = useTabsStore.getState().tabs[0];
    expect(tab?.request.params).toEqual([{ key: "page", value: "2", enabled: true }]);
  });

  it("rewrites the URL when the params table changes", () => {
    const id = useTabsStore.getState().openTab();
    useTabsStore.getState().setUrl(id, "https://a.test/users");
    useTabsStore
      .getState()
      .setParams(id, [{ key: "page", value: "3", enabled: true }]);
    expect(useTabsStore.getState().tabs[0]?.request.url).toBe("https://a.test/users?page=3");
  });

  it("removes a param from the URL when it is disabled", () => {
    const id = useTabsStore.getState().openTab();
    useTabsStore.getState().setUrl(id, "https://a.test?page=2");
    useTabsStore
      .getState()
      .setParams(id, [{ key: "page", value: "2", enabled: false }]);
    const tab = useTabsStore.getState().tabs[0];
    expect(tab?.request.url).toBe("https://a.test");
    // The row stays in the table so it can be re-enabled.
    expect(tab?.request.params).toHaveLength(1);
  });

  it("keeps a disabled param when the URL is edited", () => {
    const id = useTabsStore.getState().openTab();
    useTabsStore.getState().setParams(id, [{ key: "debug", value: "1", enabled: false }]);
    useTabsStore.getState().setUrl(id, "https://a.test?page=2");
    expect(useTabsStore.getState().tabs[0]?.request.params).toEqual([
      { key: "page", value: "2", enabled: true },
      { key: "debug", value: "1", enabled: false },
    ]);
  });

  it("marks a tab dirty once it is edited", () => {
    const id = useTabsStore.getState().openTab();
    expect(useTabsStore.getState().tabs[0]?.isDirty).toBe(false);
    useTabsStore.getState().updateRequest(id, { method: "POST" });
    expect(useTabsStore.getState().tabs[0]?.isDirty).toBe(true);
  });

  it("stores the response on the right tab", () => {
    const { openTab } = useTabsStore.getState();
    const first = openTab();
    openTab();
    useTabsStore.getState().setTabState(first, { isSending: true });
    const [a, b] = useTabsStore.getState().tabs;
    expect(a?.isSending).toBe(true);
    expect(b?.isSending).toBe(false);
  });
});

describe("titleFor", () => {
  it("prefers an explicit name", () => {
    expect(titleFor({ ...emptyRequest(), name: "List users" })).toBe("List users");
  });

  it("falls back to the URL without its scheme or query", () => {
    expect(titleFor({ ...emptyRequest(), url: "https://a.test/users?page=1" })).toBe(
      "a.test/users",
    );
  });

  it("labels an empty request", () => {
    expect(titleFor(emptyRequest())).toBe("Untitled request");
  });

  it("truncates a very long URL", () => {
    const title = titleFor({ ...emptyRequest(), url: `https://a.test/${"x".repeat(80)}` });
    expect(title).toHaveLength(40);
    expect(title.endsWith("…")).toBe(true);
  });
});
