import { LazyStore } from "@tauri-apps/plugin-store";

import type { Tab } from "./types";

export interface BrowserWorkspace {
  directoryPath: string;
  dockHeight: number;
  showTerminalDock: boolean;
  view: "files" | "terminal";
}

export interface Workspace {
  activeId: string;
  browsers: Record<string, BrowserWorkspace>;
  selectedId: string;
  tabs: Tab[];
}

export const defaultWorkspace = (): Workspace => ({
  activeId: "home",
  browsers: {},
  selectedId: "",
  tabs: [{ id: "home", title: "Home" }],
});

const workspaceStore = new LazyStore("workspace.json", { autoSave: 300 });
const workspaceKey = "workspace";

function isTab(value: unknown): value is Tab {
  if (!value || typeof value !== "object") return false;
  const tab = value as Partial<Tab>;
  return (
    typeof tab.id === "string" &&
    typeof tab.title === "string" &&
    (tab.profileId === undefined || typeof tab.profileId === "string")
  );
}

function isBrowserWorkspace(value: unknown): value is BrowserWorkspace {
  if (!value || typeof value !== "object") return false;
  const state = value as Partial<BrowserWorkspace>;
  return (
    typeof state.directoryPath === "string" &&
    typeof state.dockHeight === "number" &&
    typeof state.showTerminalDock === "boolean" &&
    (state.view === "files" || state.view === "terminal")
  );
}

export function normalizeWorkspace(value: unknown): Workspace {
  if (!value || typeof value !== "object") return defaultWorkspace();
  const saved = value as Partial<Workspace>;
  const tabs = Array.isArray(saved.tabs) ? saved.tabs.filter(isTab) : [];
  const restoredTabs = tabs.some((tab) => tab.id === "home")
    ? tabs
    : [{ id: "home", title: "Home" }, ...tabs];
  const browsers = Object.fromEntries(
    Object.entries(saved.browsers ?? {}).filter(([, state]) =>
      isBrowserWorkspace(state),
    ),
  ) as Record<string, BrowserWorkspace>;
  const activeId = restoredTabs.some((tab) => tab.id === saved.activeId)
    ? saved.activeId!
    : "home";

  return {
    activeId,
    browsers,
    selectedId: typeof saved.selectedId === "string" ? saved.selectedId : "",
    tabs: restoredTabs,
  };
}

export async function loadWorkspace(): Promise<Workspace> {
  return normalizeWorkspace(await workspaceStore.get<unknown>(workspaceKey));
}

export async function saveWorkspace(workspace: Workspace) {
  await workspaceStore.set(workspaceKey, workspace);
}
