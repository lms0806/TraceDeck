import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import type { Profile, RemoteEntry } from "./types";

type CachedDirectory = { entries: RemoteEntry[]; readAt: number };

export function useRemoteDirectory(
  profile: Profile,
  active: boolean,
  notify: (text: string) => void,
) {
  const [path, setPath] = useState(".");
  const [history, setHistory] = useState<string[]>([]);
  const [entries, setEntries] = useState<RemoteEntry[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [loading, setLoading] = useState(false);
  const requestInProgress = useRef(false);
  const queuedNavigation = useRef<(() => void) | null>(null);
  const directoryCache = useRef(new Map<string, CachedDirectory>());
  const entriesHash = useRef("");

  async function load(next: string, saveHistory: boolean, silent = false) {
    if (requestInProgress.current) {
      if (!silent)
        queuedNavigation.current = () => void load(next, saveHistory);
      return;
    }
    const cached = directoryCache.current.get(next);
    if (cached) {
      entriesHash.current = cached.entries
        .map((entry) => `${entry.path}|${entry.details}`)
        .join("\n");
      setEntries(cached.entries);
      setLoaded(true);
      if (saveHistory && next !== path) setHistory((items) => [...items, path]);
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
      if (!cached && saveHistory && next !== path)
        setHistory((items) => [...items, path]);
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

  function back() {
    const previous = history.at(-1);
    if (!previous) return;
    setHistory((items) => items.slice(0, -1));
    void load(previous, false);
  }

  return {
    path,
    setPath,
    entries,
    loaded,
    loading,
    canGoBack: history.length > 0,
    load,
    back,
  };
}
