import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { File } from "lucide-react";

import { AnsiLogContent } from "./LogDock";

export interface LogPopupSnapshot {
  path: string;
  content: string;
}

export interface LogPopupChunk {
  chunk: string;
}

export function LogPopup({ label }: { label: string }) {
  const [log, setLog] = useState<LogPopupSnapshot>({
    path: "로그를 불러오는 중…",
    content: "",
  });
  const scrollRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    const unlisteners: (() => void)[] = [];
    let disposed = false;

    void Promise.all([
      listen<LogPopupSnapshot>("log-popup-snapshot", (event) => {
        setLog(event.payload);
      }),
      listen<LogPopupChunk>("log-popup-chunk", (event) => {
        setLog((current) => {
          const lines = `${current.content}${event.payload.chunk}`.split("\n");
          return { ...current, content: lines.slice(-2_000).join("\n") };
        });
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
      unlisteners.forEach((stop) => stop());
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
          tail -F 추적 중
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
