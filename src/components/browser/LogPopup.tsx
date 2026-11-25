import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { File } from "lucide-react";

import { AnsiLogContent } from "./LogDock";
import type { Profile, TailChunk } from "./types";

export interface LogPopupSnapshot {
  path: string;
  content: string;
  profile: Profile;
}

export function LogPopup({ label }: { label: string }) {
  const [log, setLog] = useState<LogPopupSnapshot>({
    path: "로그를 불러오는 중…",
    content: "",
    profile: {
      id: "",
      name: "",
      host: "",
      port: 22,
      username: "",
      group: "",
      authKind: "privateKey",
    },
  });
  const [streamState, setStreamState] = useState<
    "connecting" | "tracking" | "failed"
  >("connecting");
  const scrollRef = useRef<HTMLPreElement>(null);
  const streamId = useRef<number | null>(null);
  const starting = useRef(false);
  const bufferedChunks = useRef(new Map<number, string>());
  const receivedOwnChunk = useRef(false);

  useEffect(() => {
    const unlisteners: (() => void)[] = [];
    let disposed = false;

    function appendChunk(chunk: string) {
      setLog((current) => {
        const previous = receivedOwnChunk.current ? current.content : "";
        receivedOwnChunk.current = true;
        const lines = `${previous}${chunk}`.split("\n");
        return { ...current, content: lines.slice(-2_000).join("\n") };
      });
    }

    async function stopStream() {
      const id = streamId.current;
      if (id === null) return;
      streamId.current = null;
      await invoke("stop_tail_stream", { streamId: id });
    }

    async function startStream(snapshot: LogPopupSnapshot) {
      if (starting.current || streamId.current !== null) return;
      starting.current = true;
      setLog(snapshot);
      try {
        const id = await invoke<number>("start_tail_stream", {
          profile: snapshot.profile,
          path: snapshot.path,
        });
        if (disposed) {
          await invoke("stop_tail_stream", { streamId: id });
          return;
        }
        streamId.current = id;
        setStreamState("tracking");
        const buffered = bufferedChunks.current.get(id);
        bufferedChunks.current.clear();
        if (buffered !== undefined) appendChunk(buffered);
      } catch (error) {
        setStreamState("failed");
        setLog((current) => ({
          ...current,
          content: `${current.content}\n\n팝업 로그 연결에 실패했습니다.\n${String(error)}`,
        }));
      } finally {
        starting.current = false;
      }
    }

    const handleUnload = () => {
      void stopStream();
    };
    window.addEventListener("beforeunload", handleUnload);

    void Promise.all([
      listen<LogPopupSnapshot>("log-popup-snapshot", (event) => {
        void startStream(event.payload);
      }),
      listen<TailChunk>("tail-chunk", (event) => {
        if (event.payload.streamId === streamId.current) {
          appendChunk(event.payload.chunk);
          return;
        }
        if (streamId.current === null) {
          const previous =
            bufferedChunks.current.get(event.payload.streamId) ?? "";
          bufferedChunks.current.set(
            event.payload.streamId,
            `${previous}${event.payload.chunk}`.slice(-100_000),
          );
        }
      }),
    ]).then((stops) => {
      if (disposed) stops.forEach((stop) => stop());
      else {
        unlisteners.push(...stops);
        void emit("log-popup-ready", { label });
      }
    });

    return () => {
      disposed = true;
      window.removeEventListener("beforeunload", handleUnload);
      unlisteners.forEach((stop) => stop());
      void stopStream();
    };
  }, [label]);

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [log.content]);

  return (
    <main className="flex h-screen min-h-0 flex-col overflow-hidden bg-[#0d141b] text-[#dce7f1]">
      <header className="flex h-10 shrink-0 items-center gap-2 border-b border-[#293d4d] px-3 text-xs text-[#ccdce7]">
        <File size={15} className="shrink-0 text-[#74d6ac]" />
        <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap font-mono">
          {log.path}
        </span>
        <small className="ml-auto shrink-0 rounded-sm bg-[#153d32] px-1.5 py-0.5 text-[10px] text-[#74d6ac]">
          {streamState === "connecting"
            ? "연결 중"
            : streamState === "tracking"
              ? "tail -F 추적 중"
              : "연결 실패"}
        </small>
      </header>
      <pre
        className="m-0 min-h-0 flex-1 overflow-auto p-[12px_16px] font-mono text-xs leading-[1.55] whitespace-pre-wrap text-[#d0e0ec]"
        ref={scrollRef}
      >
        <AnsiLogContent content={log.content} />
      </pre>
    </main>
  );
}
