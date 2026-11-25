import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

import type { LogPopupSnapshot } from "./LogPopup";
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
  const pendingChunks = useRef(new Map<string, string>());
  const flushTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const popupLabels = useRef(new Map<string, string>());

  useEffect(() => {
    logsRef.current = logs;
  }, [logs]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const chunks = pendingChunks.current;

    function flushPendingChunks() {
      flushTimer.current = null;
      if (chunks.size === 0) return;
      const pending = new Map(chunks);
      chunks.clear();
      setLogs((current) =>
        current.map((log) => {
          const chunk = pending.get(log.id);
          if (chunk === undefined) return log;
          const lines = `${log.content}${chunk}`.split("\n");
          return { ...log, content: lines.slice(-2_000).join("\n") };
        }),
      );
    }

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
      const previous = chunks.get(logId) ?? "";
      chunks.set(logId, `${previous}${event.payload.chunk}`.slice(-200_000));
      flushTimer.current ??= setTimeout(flushPendingChunks, 50);
    }).then((stop) => {
      unlisten = stop;
    });
    return () => {
      unlisten?.();
      if (flushTimer.current !== null) clearTimeout(flushTimer.current);
      flushTimer.current = null;
      chunks.clear();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<{ label: string }>("log-popup-ready", (event) => {
      const entry = [...popupLabels.current.entries()].find(
        ([, label]) => label === event.payload.label,
      );
      if (!entry) return;
      const log = logsRef.current.find((item) => item.id === entry[0]);
      if (!log) return;
      const snapshot: LogPopupSnapshot = {
        path: log.path,
        content: log.content,
        profile,
      };
      void emitTo(event.payload.label, "log-popup-snapshot", snapshot);
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  }, [profile]);

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
    pendingChunks.current.delete(id);
    const popupLabel = popupLabels.current.get(id);
    if (popupLabel !== undefined) {
      popupLabels.current.delete(id);
      void WebviewWindow.getByLabel(popupLabel).then((window) =>
        window?.close(),
      );
    }
    const remaining = logsRef.current.filter((item) => item.id !== id);
    logsRef.current = remaining;
    setLogs(remaining);
    if (activeLogId === id) {
      if (remaining.length === 0) setActiveLogId(null);
      else setActiveLogId(remaining[Math.max(0, index - 1)].id);
    }
  }

  async function popoutLog(id: string) {
    const log = logsRef.current.find((item) => item.id === id);
    if (!log) return;

    const existingLabel = popupLabels.current.get(id);
    if (existingLabel !== undefined) {
      const existing = await WebviewWindow.getByLabel(existingLabel);
      if (existing) {
        await existing.setFocus();
        return;
      }
      popupLabels.current.delete(id);
    }

    const label = `log-${crypto.randomUUID()}`;
    popupLabels.current.set(id, label);
    const filename = log.path.split("/").pop() ?? log.path;
    const popup = new WebviewWindow(label, {
      url: `index.html?logPopup=${encodeURIComponent(label)}`,
      title: `TraceDeck - ${filename}`,
      width: 1_000,
      height: 700,
      minWidth: 600,
      minHeight: 400,
    });
    void popup.once("tauri://error", () => {
      if (popupLabels.current.get(id) === label) popupLabels.current.delete(id);
    });
    void popup.once("tauri://destroyed", () => {
      if (popupLabels.current.get(id) === label) popupLabels.current.delete(id);
    });
  }

  const activeLog = logs.find((log) => log.id === activeLogId) ?? null;
  return {
    activeLog,
    activeLogId,
    closeLog,
    logs,
    openFile,
    popoutLog,
    setActiveLogId,
  };
}
