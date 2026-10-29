import {
  CSSProperties,
  useLayoutEffect,
  useRef,
  type PointerEvent,
} from "react";
import { File, X } from "lucide-react";

import { IconButton } from "../ui/IconButton";

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
  onOpenTerminal,
  height,
  onHeightChange,
}: {
  preview: RemoteFile;
  streamId: number | null;
  onClose: () => void;
  onOpenTerminal: () => void;
  height: number;
  onHeightChange: (height: number) => void;
}) {
  const previewScroll = useRef<HTMLPreElement>(null);
  const stickToBottom = useRef(true);
  const scrollFrame = useRef<number | null>(null);

  useLayoutEffect(() => {
    if (!stickToBottom.current) return;
    if (scrollFrame.current !== null) cancelAnimationFrame(scrollFrame.current);
    scrollFrame.current = requestAnimationFrame(() => {
      const element = previewScroll.current;
      if (element && stickToBottom.current)
        element.scrollTop = element.scrollHeight;
      scrollFrame.current = null;
    });
    return () => {
      if (scrollFrame.current !== null)
        cancelAnimationFrame(scrollFrame.current);
    };
  }, [preview.content]);

  function beginResize(event: PointerEvent<HTMLDivElement>) {
    event.preventDefault();
    const startY = event.clientY;
    const move = (moveEvent: globalThis.PointerEvent) => {
      onHeightChange(
        Math.max(
          150,
          Math.min(
            window.innerHeight * 0.75,
            height + startY - moveEvent.clientY,
          ),
        ),
      );
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
  }

  return (
    <section
      className="file-preview-dock flex min-h-[150px] max-h-[75vh] flex-none flex-col overflow-hidden border-t-[3px] border-[#3a5264] bg-[#0d141b]"
      style={{ height }}
    >
      <div className="dock-resize-handle" onPointerDown={beginResize} />
      <header className="flex h-[38px] items-center justify-between border-b border-[#293d4d] px-3 text-xs text-[#ccdce7]">
        <div className="flex min-w-0 items-center gap-1">
          <button className="flex min-w-0 items-center gap-[7px] rounded bg-[#263b4b] px-2 py-1 text-[#f1f7fb]">
            <File size={15} />
            <span className="overflow-hidden text-ellipsis whitespace-nowrap">
              {preview.path}
            </span>
            <small className="whitespace-nowrap rounded-sm bg-[#153d32] px-[5px] py-0.5 text-[10px] text-[#74d6ac]">
              {streamId === null ? "준비 중" : "tail -F 추적 중"}
            </small>
          </button>
          <button
            className="rounded px-2 py-1 text-[#9fb2c2] hover:bg-[#263746] hover:text-white"
            onClick={onOpenTerminal}
          >
            터미널
          </button>
        </div>
        <div className="flex items-center gap-1">
          <IconButton onClick={onClose}>
            <X size={18} />
          </IconButton>
        </div>
      </header>
      <pre
        className="m-0 min-h-0 flex-1 overflow-auto p-[12px_16px] font-mono text-xs leading-[1.55] whitespace-pre-wrap text-[#d0e0ec]"
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
