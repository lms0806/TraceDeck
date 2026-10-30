import type { CSSProperties } from "react";
import { useLayoutEffect, useRef, useState, type PointerEvent } from "react";
import { Columns2, File, X } from "lucide-react";

import type { LogTab } from "./useTailStream";

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

function LogPreview({ log }: { log: LogTab }) {
  const previewScroll = useRef<HTMLPreElement>(null);
  const stickToBottom = useRef(true);

  useLayoutEffect(() => {
    if (!stickToBottom.current) return;
    const frame = requestAnimationFrame(() => {
      const element = previewScroll.current;
      if (element && stickToBottom.current)
        element.scrollTop = element.scrollHeight;
    });
    return () => cancelAnimationFrame(frame);
  }, [log.content]);

  return (
    <section className="flex min-w-0 flex-1 flex-col overflow-hidden">
      <div className="shrink-0 border-b border-[#293d4d] px-3 py-1.5 font-mono text-[10px] text-[#89a1b2]">
        {log.path}
      </div>
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
        <AnsiLogContent content={log.content} />
      </pre>
    </section>
  );
}

export function LogDock({
  logs,
  activeLogId,
  splitLogIds,
  onCloseLog,
  onSelectLog,
  onSplitLogIdsChange,
  onOpenTerminal,
  height,
  onHeightChange,
}: {
  logs: LogTab[];
  activeLogId: string;
  splitLogIds: [string, string] | null;
  onCloseLog: (id: string) => void;
  onSelectLog: (id: string) => void;
  onSplitLogIdsChange: (ids: [string, string] | null) => void;
  onOpenTerminal: () => void;
  height: number;
  onHeightChange: (height: number) => void;
}) {
  const preview = logs.find((log) => log.id === activeLogId);
  const [dropSide, setDropSide] = useState<"left" | "right" | null>(null);
  const dockRef = useRef<HTMLElement>(null);
  const draggedLogId = useRef<string | null>(null);
  const hasMoved = useRef(false);
  const pointerStart = useRef<{ x: number; y: number } | null>(null);
  const suppressClick = useRef(false);

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

  function getDropSide(clientX: number, clientY: number) {
    const bounds = dockRef.current?.getBoundingClientRect();
    if (
      !bounds ||
      clientX < bounds.left ||
      clientX > bounds.right ||
      clientY < bounds.top ||
      clientY > bounds.bottom
    )
      return null;
    return clientX < bounds.left + bounds.width / 2 ? "left" : "right";
  }

  function beginTabDrag(event: PointerEvent<HTMLButtonElement>, id: string) {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    draggedLogId.current = id;
    hasMoved.current = false;
    pointerStart.current = { x: event.clientX, y: event.clientY };
  }

  function moveTabDrag(event: PointerEvent<HTMLButtonElement>) {
    const start = pointerStart.current;
    if (start === null || draggedLogId.current === null) return;
    if (Math.hypot(event.clientX - start.x, event.clientY - start.y) < 6)
      return;
    hasMoved.current = true;
    setDropSide(getDropSide(event.clientX, event.clientY));
  }

  function endTabDrag(event: PointerEvent<HTMLButtonElement>) {
    const draggedId = draggedLogId.current;
    const side = getDropSide(event.clientX, event.clientY);
    const dragged = hasMoved.current;
    draggedLogId.current = null;
    hasMoved.current = false;
    pointerStart.current = null;
    setDropSide(null);
    if (
      !preview ||
      draggedId === null ||
      side === null ||
      !dragged ||
      draggedId === preview.id
    )
      return;
    suppressClick.current = true;
    onSplitLogIdsChange(
      side === "left" ? [draggedId, preview.id] : [preview.id, draggedId],
    );
  }

  const splitLogs = splitLogIds
    ?.map((id) => logs.find((log) => log.id === id))
    .filter((log): log is LogTab => log !== undefined);
  const splitTabLabel = splitLogs
    ?.map((log) => log.path.split("/").pop() ?? log.path)
    .join(" + ");

  if (!preview) return null;

  return (
    <section
      className={`file-preview-dock relative flex min-h-[150px] max-h-[75vh] flex-none flex-col overflow-hidden border-t-[3px] border-[#3a5264] bg-[#0d141b] ${
        dropSide ? "ring-2 ring-inset ring-[#50d3a1]" : ""
      }`}
      ref={dockRef}
      style={{ height }}
    >
      <div className="dock-resize-handle" onPointerDown={beginResize} />
      <header className="flex h-[38px] items-center justify-between border-b border-[#293d4d] px-3 text-xs text-[#ccdce7]">
        <div className="flex min-w-0 items-center gap-1">
          {logs.map((log) => {
            const isSplitLog = splitLogIds?.includes(log.id) ?? false;
            if (isSplitLog) {
              if (log.id !== splitLogIds?.[0]) return null;
              return (
                <div
                  className="flex min-w-0 items-center gap-[7px] rounded bg-[#263b4b] px-2 py-1 text-[#f1f7fb] ring-1 ring-inset ring-[#50d3a1]"
                  key="split-log-tab"
                  title="분할 보기"
                >
                  <Columns2 size={15} />
                  <span className="max-w-[280px] overflow-hidden text-ellipsis whitespace-nowrap">
                    {splitTabLabel ?? "분할 보기"}
                  </span>
                  <small className="flex items-center gap-0.5 whitespace-nowrap rounded-sm bg-[#174b3b] px-[5px] py-0.5 text-[10px] text-[#a6ffce]">
                    분할
                  </small>
                  <button
                    aria-label="분할 보기 닫기"
                    className="shrink-0 rounded p-0.5 hover:bg-[#2c6653]"
                    onClick={() => onSplitLogIdsChange(null)}
                    type="button"
                  >
                    <X size={14} />
                  </button>
                </div>
              );
            }
            return (
              <div
                className={`flex min-w-0 items-center gap-1 rounded px-2 py-1 ${
                  log.id === activeLogId
                    ? "bg-[#263b4b] text-[#f1f7fb]"
                    : "text-[#9fb2c2] hover:bg-[#263746] hover:text-white"
                }`}
                key={log.id}
              >
                <button
                  aria-label={log.path}
                  className="flex min-w-0 items-center gap-[7px]"
                  onClick={() => {
                    if (suppressClick.current) {
                      suppressClick.current = false;
                      return;
                    }
                    onSelectLog(log.id);
                  }}
                  onPointerDown={(event) => beginTabDrag(event, log.id)}
                  onPointerMove={moveTabDrag}
                  onPointerUp={endTabDrag}
                  onPointerCancel={() => {
                    draggedLogId.current = null;
                    hasMoved.current = false;
                    pointerStart.current = null;
                    setDropSide(null);
                  }}
                >
                  <File size={15} />
                  <span className="max-w-[220px] overflow-hidden text-ellipsis whitespace-nowrap">
                    {log.path}
                  </span>
                  <small className="whitespace-nowrap rounded-sm bg-[#153d32] px-[5px] py-0.5 text-[10px] text-[#74d6ac]">
                    {log.streamId === null ? "준비 중" : "tail -F 추적 중"}
                  </small>
                </button>
                <button
                  aria-label={`${log.path} 닫기`}
                  className="shrink-0 rounded p-0.5 hover:bg-[#3a5264]"
                  onClick={() => onCloseLog(log.id)}
                  type="button"
                >
                  <X size={14} />
                </button>
              </div>
            );
          })}
          <button
            className="rounded px-2 py-1 text-[#9fb2c2] hover:bg-[#263746] hover:text-white"
            onClick={onOpenTerminal}
          >
            터미널
          </button>
        </div>
        <div className="flex items-center gap-1" />
      </header>
      {splitLogs?.length === 2 ? (
        <div className="flex min-h-0 flex-1">
          <LogPreview log={splitLogs[0]} />
          <div className="w-px shrink-0 bg-[#3a5264]" />
          <LogPreview log={splitLogs[1]} />
        </div>
      ) : (
        <LogPreview log={preview} />
      )}
      {dropSide && (
        <div className="pointer-events-none absolute inset-x-0 bottom-0 top-[46px] flex bg-[#0b171ed9] p-3">
          <div
            className={`flex flex-1 items-center justify-center border-2 text-xs font-bold ${
              dropSide === "left"
                ? "border-[#50d3a1] bg-[#174b3b] text-[#a6ffce]"
                : "border-[#496373] text-[#86a0b1]"
            }`}
          >
            왼쪽으로 분할
          </div>
          <div
            className={`flex flex-1 items-center justify-center border-y-2 border-r-2 text-xs font-bold ${
              dropSide === "right"
                ? "border-[#50d3a1] bg-[#174b3b] text-[#a6ffce]"
                : "border-[#496373] text-[#86a0b1]"
            }`}
          >
            오른쪽으로 분할
          </div>
        </div>
      )}
    </section>
  );
}
