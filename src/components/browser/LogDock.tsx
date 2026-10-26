import { CSSProperties, useEffect, useRef } from "react";
import { File, X } from "lucide-react";

import type { RemoteFile } from "./types";

const ansiPalette: Record<number, string> = {
  30: "#4c5967",
  31: "#ff7b72",
  32: "#78dba9",
  33: "#e9c46a",
  34: "#78b8ff",
  35: "#d2a8ff",
  36: "#72d4de",
  37: "#d9e3ec",
  90: "#718096",
  91: "#ff938c",
  92: "#8be9b0",
  93: "#ffd47a",
  94: "#9ac7ff",
  95: "#e0b4ff",
  96: "#8de8ed",
  97: "#ffffff",
};

function AnsiLogContent({ content }: { content: string }) {
  return (
    <>
      {content.split("\n").map((line, lineIndex) => {
        const segments: { text: string; style: CSSProperties }[] = [];
        let cursor = 0;
        let style: CSSProperties = { color: "#d0e0ec" };
        // eslint-disable-next-line no-control-regex -- ANSI SGR sequences from terminal output.
        const pattern = /\u001b\[([0-9;]*)m/g;
        let match: RegExpExecArray | null;
        while ((match = pattern.exec(line))) {
          if (match.index > cursor)
            segments.push({
              text: line.slice(cursor, match.index),
              style: { ...style },
            });
          for (const code of match[1] ? match[1].split(";").map(Number) : [0]) {
            if (code === 0) style = { color: "#d0e0ec" };
            else if (code === 1) style = { ...style, fontWeight: 700 };
            else if (code === 2) style = { ...style, opacity: 0.65 };
            else if (code === 22)
              style = { ...style, fontWeight: "normal", opacity: 1 };
            else if (code === 39) style = { ...style, color: "#d0e0ec" };
            else if (ansiPalette[code])
              style = { ...style, color: ansiPalette[code] };
          }
          cursor = pattern.lastIndex;
        }
        if (cursor < line.length)
          segments.push({ text: line.slice(cursor), style: { ...style } });
        return (
          <span key={`${lineIndex}-${line}`} className="ansi-line">
            {segments.map((segment, index) => (
              <span key={index} style={segment.style}>
                {segment.text}
              </span>
            ))}
            {"\n"}
          </span>
        );
      })}
    </>
  );
}

export function LogDock({
  preview,
  streamId,
  onClose,
}: {
  preview: RemoteFile;
  streamId: number | null;
  onClose: () => void;
}) {
  const dockRef = useRef<HTMLElement>(null);
  const previewScroll = useRef<HTMLPreElement>(null);
  const stickToBottom = useRef(true);

  useEffect(() => {
    const element = previewScroll.current;
    if (element && stickToBottom.current)
      element.scrollTop = element.scrollHeight;
  }, [preview.content]);

  useEffect(() => {
    const dock = dockRef.current;
    if (!dock) return;
    const handle = document.createElement("div");
    handle.className = "dock-resize-handle";
    const begin = (event: PointerEvent) => {
      event.preventDefault();
      const startY = event.clientY;
      const startHeight = dock.getBoundingClientRect().height;
      const move = (moveEvent: PointerEvent) => {
        dock.style.height = `${Math.max(150, Math.min(window.innerHeight * 0.75, startHeight + startY - moveEvent.clientY))}px`;
      };
      const stop = () => {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", stop);
      };
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", stop);
    };
    handle.addEventListener("pointerdown", begin);
    dock.prepend(handle);
    return () => {
      handle.removeEventListener("pointerdown", begin);
      handle.remove();
    };
  }, []);

  return (
    <section className="file-preview-dock" ref={dockRef}>
      <header>
        <div>
          <File size={16} />
          <span>{preview.path}</span>
          <small>{streamId === null ? "준비 중" : "tail -F 추적 중"}</small>
        </div>
        <button onClick={onClose}>
          <X size={18} />
        </button>
      </header>
      <pre
        ref={previewScroll}
        onScroll={(event) => {
          const element = event.currentTarget;
          stickToBottom.current =
            element.scrollHeight - element.scrollTop - element.clientHeight <
            24;
        }}
      >
        <AnsiLogContent content={preview.content} />
      </pre>
    </section>
  );
}
