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
  const streamLogs = useRef(new Map<number, string>());
  const bufferedChunks = useRef(new Map<number, string>());

  useEffect(() => {
    logsRef.current = logs;
  }, [logs]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<TailChunk>("tail-chunk", (event) => {
      const logId = streamLogs.current.get(event.payload.streamId);
      if (logId === undefined) {
        const previous =
          bufferedChunks.current.get(event.payload.streamId) ?? "";
        bufferedChunks.current.set(
          event.payload.streamId,
          `${previous}${event.payload.chunk}`.slice(-100_000),
        );
        return;
      }
      setLogs((current) =>
        current.map((log) => {
          if (log.id !== logId) return log;
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
      for (const streamId of streamLogs.current.keys())
        void invoke("stop_tail_stream", { streamId });
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
      const streamId = await invoke<number>("start_tail_stream", {
        profile,
        path: entry.path,
      });
      if (!logsRef.current.some((log) => log.id === id)) {
        void invoke("stop_tail_stream", { streamId });
        return;
      }
      streamLogs.current.set(streamId, id);
      const buffered = bufferedChunks.current.get(streamId) ?? "";
      bufferedChunks.current.delete(streamId);
      setLogs((current) =>
        current.map((log) => {
          if (log.id !== id) return log;
          const lines = buffered.split("\n");
          return {
            ...log,
            content: lines.slice(-2_000).join("\n"),
            streamId,
          };
        }),
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
    const streamId =
      log.streamId ??
      [...streamLogs.current].find(([, logId]) => logId === id)?.[0] ??
      null;
    if (streamId !== null) {
      streamLogs.current.delete(streamId);
      bufferedChunks.current.delete(streamId);
      void invoke("stop_tail_stream", { streamId });
    }
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
