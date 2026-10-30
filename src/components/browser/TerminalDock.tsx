import { useEffect, useRef, type PointerEvent } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal as Xterm } from "@xterm/xterm";
import { File, Terminal, Trash2, X } from "lucide-react";
import "@xterm/xterm/css/xterm.css";

import { IconButton } from "../ui/IconButton";

interface TerminalState {
  clear: () => void;
  output: string;
  resize: (columns: number, rows: number) => void;
  starting: boolean;
  terminalId: number | null;
  write: (input: string) => Promise<void>;
}

export function TerminalDock({
  terminal,
  onClose,
  logPath,
  onShowLog,
  height,
  onHeightChange,
  expanded = false,
}: {
  terminal: TerminalState;
  onClose: () => void;
  logPath?: string;
  onShowLog?: () => void;
  height?: number;
  onHeightChange?: (height: number) => void;
  expanded?: boolean;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Xterm | null>(null);
  const renderedOutput = useRef("");
  const terminalRef = useRef(terminal);

  useEffect(() => {
    terminalRef.current = terminal;
  }, [terminal]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const xterm = new Xterm({
      convertEol: true,
      cursorBlink: true,
      fontFamily: "Cascadia Mono, Consolas, monospace",
      fontSize: 13,
      theme: {
        background: "#0d141b",
        cursor: "#dce7f1",
        foreground: "#d0e0ec",
        selectionBackground: "#3a668c80",
      },
    });
    const fitAddon = new FitAddon();
    xterm.loadAddon(fitAddon);
    xterm.open(container);
    xtermRef.current = xterm;
    const fit = () => {
      fitAddon.fit();
      terminalRef.current.resize(xterm.cols, xterm.rows);
    };
    requestAnimationFrame(() => {
      fit();
      xterm.focus();
    });
    const resizeObserver = new ResizeObserver(fit);
    resizeObserver.observe(container);
    const inputSubscription = xterm.onData((input) => {
      void terminalRef.current.write(input);
    });
    const copySelection = () => {
      const selection = xterm.getSelection();
      if (selection) void navigator.clipboard.writeText(selection);
    };
    const pasteClipboard = () => {
      void navigator.clipboard.readText().then((text) => {
        if (text) return terminalRef.current.write(text);
      });
    };
    const handleClipboardShortcut = (event: KeyboardEvent) => {
      if (!event.ctrlKey && !event.metaKey) return;
      const key = event.key.toLowerCase();
      if (key === "c" && xterm.hasSelection()) {
        event.preventDefault();
        copySelection();
      }
      if (key === "v") {
        event.preventDefault();
        pasteClipboard();
      }
    };
    container.addEventListener("keydown", handleClipboardShortcut, true);
    return () => {
      inputSubscription.dispose();
      resizeObserver.disconnect();
      container.removeEventListener("keydown", handleClipboardShortcut, true);
      xterm.dispose();
      xtermRef.current = null;
    };
  }, []);

  useEffect(() => {
    const xterm = xtermRef.current;
    if (!xterm) return;
    const previous = renderedOutput.current;
    if (terminal.output.startsWith(previous)) {
      xterm.write(terminal.output.slice(previous.length));
    } else {
      xterm.reset();
      xterm.write(terminal.output);
    }
    renderedOutput.current = terminal.output;
  }, [terminal.output]);

  function beginResize(event: PointerEvent<HTMLDivElement>) {
    if (!onHeightChange || height === undefined) return;
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
      className={`file-preview-dock flex min-h-0 flex-col overflow-hidden border-t-[3px] border-[#3a5264] bg-[#0d141b] ${
        expanded ? "flex-1" : "min-h-[150px] max-h-[75vh] flex-none"
      }`}
      style={expanded ? undefined : { height }}
    >
      {!expanded && (
        <div className="dock-resize-handle" onPointerDown={beginResize} />
      )}
      <header className="flex h-[38px] items-center justify-between border-b border-[#293d4d] px-3 text-xs text-[#ccdce7]">
        <div className="flex min-w-0 items-center gap-1">
          {logPath !== undefined && onShowLog !== undefined && (
            <button
              className="flex min-w-0 items-center gap-1 rounded px-2 py-1 text-[#9fb2c2] hover:bg-[#263746] hover:text-white"
              onClick={onShowLog}
            >
              <File size={15} />
              <span className="max-w-[260px] overflow-hidden text-ellipsis whitespace-nowrap">
                {logPath}
              </span>
            </button>
          )}
          <span className="flex items-center gap-[7px] rounded bg-[#263b4b] px-2 py-1 text-[#f1f7fb]">
            <Terminal size={16} />
            <span>터미널</span>
            <small className="rounded-sm bg-[#153d32] px-[5px] py-0.5 text-[10px] text-[#74d6ac]">
              {terminal.starting || terminal.terminalId === null
                ? "연결 중"
                : "연결됨"}
            </small>
          </span>
        </div>
        <div className="flex items-center gap-1">
          <IconButton aria-label="터미널 출력 지우기" onClick={terminal.clear}>
            <Trash2 size={15} />
          </IconButton>
          <IconButton aria-label="터미널 닫기" onClick={onClose}>
            <X size={18} />
          </IconButton>
        </div>
      </header>
      <div className="min-h-0 flex-1 p-2" ref={containerRef} />
    </section>
  );
}
