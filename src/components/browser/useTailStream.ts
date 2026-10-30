import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { Profile, RemoteEntry, RemoteFile, TailChunk } from "./types";

export interface LogTab extends RemoteFile {
  id: string;
  streamId: number | null;
}

export function useTailStream(profile: Profile) {
  const [logs, setLogs] = useState<LogTab[]>([]);
  const [activeLogId, setActiveLogId] = useState<string | null>(null);
  const logsRef = useRef<LogTab[]>([]);

  useEffect(() => {
    logsRef.current = logs;
  }, [logs]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<TailChunk>("tail-chunk", (event) => {
      setLogs((current) =>
        current.map((log) => {
          if (log.streamId !== event.payload.streamId) return log;
          const lines = `${log.content}${event.payload.chunk}`.split("\n");
          return { ...log, content: lines.slice(-2_000).join("\n") };
        }),
      );
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  }, []);

  useEffect(
    () => () => {
      for (const log of logsRef.current) {
        if (log.streamId !== null)
          void invoke("stop_tail_stream", { streamId: log.streamId });
      }
    },
    [],
  );

  async function openFile(entry: RemoteEntry) {
    const existing = logsRef.current.find((log) => log.path === entry.path);
    if (existing) {
      setActiveLogId(existing.id);
      return;
    }

    const id = crypto.randomUUID();
    const loadingTab: LogTab = {
      id,
      path: entry.path,
      content: "마지막 100줄을 불러오는 중…\n",
      truncated: false,
      streamId: null,
    };
    logsRef.current = [...logsRef.current, loadingTab];
    setLogs((current) => [...current, loadingTab]);
    setActiveLogId(id);
    try {
      const initial = await invoke<RemoteFile>("read_remote_file", {
        profile,
        path: entry.path,
      });
      setLogs((current) =>
        current.map((log) => (log.id === id ? { ...log, ...initial } : log)),
      );
      const streamId = await invoke<number>("start_tail_stream", {
        profile,
        path: entry.path,
      });
      if (!logsRef.current.some((log) => log.id === id)) {
        void invoke("stop_tail_stream", { streamId });
        return;
      }
      setLogs((current) =>
        current.map((log) => (log.id === id ? { ...log, streamId } : log)),
      );
    } catch (error) {
      setLogs((current) =>
        current.map((log) =>
          log.id === id
            ? {
                ...log,
                content: `실시간 로그를 열지 못했습니다.\n${String(error)}`,
              }
            : log,
        ),
      );
    }
  }

  function closeLog(id: string) {
    const index = logsRef.current.findIndex((log) => log.id === id);
    if (index === -1) return;
    const log = logsRef.current[index];
    if (log.streamId !== null)
      void invoke("stop_tail_stream", { streamId: log.streamId });
    const remaining = logsRef.current.filter((item) => item.id !== id);
    logsRef.current = remaining;
    setLogs(remaining);
    if (activeLogId === id) {
      if (remaining.length === 0) setActiveLogId(null);
      else setActiveLogId(remaining[Math.max(0, index - 1)].id);
    }
  }

  const activeLog = logs.find((log) => log.id === activeLogId) ?? null;
  return { activeLog, activeLogId, closeLog, logs, openFile, setActiveLogId };
}
