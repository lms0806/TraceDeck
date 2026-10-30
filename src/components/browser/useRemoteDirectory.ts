import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import type { Profile, RemoteEntry } from "./types";

interface CachedDirectory {
  entries: RemoteEntry[];
  readAt: number;
}

export function useRemoteDirectory(
  profile: Profile,
  active: boolean,
  notify: (text: string) => void,
  initialPath = ".",
) {
  const [path, setPath] = useState(initialPath);
  const [entries, setEntries] = useState<RemoteEntry[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [loading, setLoading] = useState(false);
  const requestInProgress = useRef(false);
  const queuedNavigation = useRef<(() => void) | null>(null);
  const directoryCache = useRef(new Map<string, CachedDirectory>());
  const entriesHash = useRef("");

  async function load(
    next: string,
    _saveHistory: boolean,
    silent = false,
    force = false,
  ) {
    if (requestInProgress.current) {
      if (!silent)
        queuedNavigation.current = () =>
          void load(next, _saveHistory, false, force);
      return;
    }
    const cached = force ? undefined : directoryCache.current.get(next);
    if (cached) {
      entriesHash.current = cached.entries
        .map((entry) => `${entry.path}|${entry.details}`)
        .join("\n");
      setEntries(cached.entries);
      setLoaded(true);
      setPath(next);
      if (Date.now() - cached.readAt < 8_000) return;
      silent = true;
    }

    requestInProgress.current = true;
    if (!silent) {
      setLoading(true);
      setLoaded(true);
    }
    try {
      const result = await invoke<RemoteEntry[]>("list_remote_directory", {
        profile,
        path: next,
      });
      const hash = result
        .map((entry) => `${entry.path}|${entry.details}`)
        .join("\n");
      if (hash !== entriesHash.current) {
        entriesHash.current = hash;
        setEntries(result);
      }
      directoryCache.current.set(next, { entries: result, readAt: Date.now() });
      setPath(next);
      if (!silent)
        notify(`${next} 폴더에서 ${result.length}개 항목을 읽었습니다.`);
    } catch (error) {
      if (!silent) notify(`SFTP 폴더를 열지 못했습니다: ${String(error)}`);
    } finally {
      if (!silent) setLoading(false);
      requestInProgress.current = false;
      const queued = queuedNavigation.current;
      queuedNavigation.current = null;
      queued?.();
    }
  }

  useEffect(() => {
    if (!loaded || !active) return;
    const timer = window.setInterval(() => void load(path, false, true), 8_000);
    return () => window.clearInterval(timer);
    // load deliberately owns mutable request and cache refs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, loaded, path, profile]);

  function parentPath(currentPath: string) {
    if (currentPath === "/") return "/";
    const normalized = currentPath.replace(/\/+$/, "");
    if (normalized === ".") return "..";
    if (normalized === "..") return "../..";
    if (normalized.split("/").every((segment) => segment === ".."))
      return `${normalized}/..`;
    const separator = normalized.lastIndexOf("/");
    if (separator < 0) return "..";
    if (separator === 0) return "/";
    return normalized.slice(0, separator);
  }

  function parent() {
    void load(parentPath(path), false);
  }

  function refresh() {
    return load(path, false, false, true);
  }

  return {
    path,
    setPath,
    entries,
    loaded,
    loading,
    load,
    refresh,
    parent,
    canOpenParent: path !== "/",
  };
}
