import { CSSProperties, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ChevronLeft,
  File,
  Folder,
  FolderOpen,
  FolderTree,
  HardDrive,
  X,
} from "lucide-react";

type Profile = {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  group: string;
  authKind?: "password" | "privateKey" | "agent";
  privateKeyPath?: string;
};
type RemoteEntry = {
  name: string;
  path: string;
  isDir: boolean;
  details: string;
};
type RemoteFile = { path: string; content: string; truncated: boolean };
type TailChunk = { streamId: number; chunk: string };

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

function LogContent({ content }: { content: string }) {
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

export function Browser({
  profile,
  active,
  onCheck,
  notify,
}: {
  profile: Profile;
  active: boolean;
  onCheck: () => void;
  notify: (text: string) => void;
}) {
  const [path, setPath] = useState(".");
  const [history, setHistory] = useState<string[]>([]);
  const [entries, setEntries] = useState<RemoteEntry[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [preview, setPreview] = useState<RemoteFile | null>(null);
  const [streamId, setStreamId] = useState<number | null>(null);
  const requestInProgress = useRef(false);
  const queuedNavigation = useRef<(() => void) | null>(null);
  const directoryCache = useRef(
    new Map<string, { entries: RemoteEntry[]; readAt: number }>(),
  );
  const entriesHash = useRef("");
  const previewStickToBottom = useRef(true);
  const previewScroll = useRef<HTMLPreElement>(null);
  async function load(next: string, saveHistory: boolean, silent = false) {
    if (requestInProgress.current) {
      if (!silent)
        queuedNavigation.current = () => {
          void load(next, saveHistory);
        };
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
  // load deliberately owns mutable request and cache refs; recreating it would reset navigation behavior.
  useEffect(() => {
    if (!loaded || !active) return;
    const timer = window.setInterval(() => {
      void load(path, false, true);
    }, 8_000);
    return () => window.clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, loaded, path, profile]);
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<TailChunk>("tail-chunk", (event) => {
      if (event.payload.streamId !== streamId) return;
      setPreview((current) => {
        if (!current) return current;
        const lines = `${current.content}${event.payload.chunk}`.split("\n");
        return { ...current, content: lines.slice(-2000).join("\n") };
      });
    }).then((stop) => {
      unlisten = stop;
    });
    return () => {
      unlisten?.();
    };
  }, [streamId]);
  useEffect(
    () => () => {
      if (streamId !== null) void invoke("stop_tail_stream", { streamId });
    },
    [streamId],
  );
  useEffect(() => {
    const element = previewScroll.current;
    if (element && previewStickToBottom.current)
      element.scrollTop = element.scrollHeight;
  }, [preview?.content]);
  useEffect(() => {
    const dock = document.querySelector<HTMLElement>(".file-preview-dock");
    if (!dock) return;
    const handle = document.createElement("div");
    handle.className = "dock-resize-handle";
    const begin = (event: PointerEvent) => {
      event.preventDefault();
      const startY = event.clientY;
      const startHeight = dock.getBoundingClientRect().height;
      const move = (moveEvent: PointerEvent) => {
        const height = Math.max(
          150,
          Math.min(
            window.innerHeight * 0.75,
            startHeight + startY - moveEvent.clientY,
          ),
        );
        dock.style.height = `${height}px`;
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
  }, [preview?.path]);
  async function showFile(entry: RemoteEntry) {
    if (streamId !== null)
      await invoke("stop_tail_stream", { streamId }).catch(() => undefined);
    previewStickToBottom.current = true;
    setPreview({
      path: entry.path,
      content: "마지막 100줄을 불러오는 중…\n",
      truncated: false,
    });
    try {
      const initial = await invoke<RemoteFile>("read_remote_file", {
        profile,
        path: entry.path,
      });
      setPreview(initial);
      setStreamId(
        await invoke<number>("start_tail_stream", {
          profile,
          path: entry.path,
        }),
      );
    } catch (error) {
      setPreview({
        path: entry.path,
        content: `실시간 로그를 열지 못했습니다.\n${String(error)}`,
        truncated: false,
      });
      setStreamId(null);
    }
  }
  function back() {
    const previous = history.at(-1);
    if (!previous) return;
    setHistory((items) => items.slice(0, -1));
    void load(previous, false);
  }
  return (
    <section className="file-workspace">
      <header className="server-header">
        <div>
          <span className="offline-dot" /> {profile.username}@{profile.host}:
          {profile.port}
        </div>
        <button className="connect-button" onClick={onCheck}>
          연결 확인
        </button>
      </header>
      <div className="file-toolbar">
        <button onClick={back} disabled={loading || history.length === 0}>
          <ChevronLeft size={16} /> 뒤로
        </button>
        <button onClick={() => load(path, false)} disabled={loading}>
          새로고침
        </button>
        <label>
          <span>원격 경로</span>
          <input
            value={path}
            onChange={(event) => setPath(event.target.value)}
            onKeyDown={(event) => event.key === "Enter" && load(path, true)}
          />
        </label>
        <button
          className="primary"
          onClick={() => load(path, true)}
          disabled={loading}
        >
          {loading ? "불러오는 중…" : "폴더 열기"}
        </button>
      </div>
      <div className="file-content">
        <aside className="folder-sidebar">
          <header>
            <FolderTree size={16} /> 위치
          </header>
          <button onClick={() => load(".", true)}>
            <Folder size={15} /> 홈
          </button>
          <button onClick={() => load("/", true)}>
            <HardDrive size={15} /> /
          </button>
          <p>목록은 변경된 경우에만 갱신됩니다.</p>
        </aside>
        <section className="file-list">
          <header>
            <div>
              <span>이름</span>
              <span>권한 · 소유자 · 크기</span>
            </div>
          </header>
          {!loaded && (
            <div className="file-empty">
              <FolderOpen size={34} />
              <strong>원격 폴더를 열 준비가 됐습니다.</strong>
              <span>“폴더 열기”를 누르면 서버의 파일 목록을 가져옵니다.</span>
            </div>
          )}
          {loaded && !loading && entries.length === 0 && (
            <div className="file-empty">
              <FolderOpen size={34} />
              <strong>폴더가 비어 있거나 읽을 수 없습니다.</strong>
            </div>
          )}
          {entries.map((entry) => (
            <button
              className="file-row"
              key={entry.path}
              onClick={() =>
                entry.isDir ? load(entry.path, true) : showFile(entry)
              }
              title={entry.details}
            >
              <span>
                {entry.isDir ? (
                  <Folder className="folder-icon" size={17} />
                ) : (
                  <File size={17} />
                )}
                <b>{entry.name}</b>
              </span>
              <small>{entry.details}</small>
            </button>
          ))}
        </section>
      </div>
      {preview && (
        <section className="file-preview-dock">
          <header>
            <div>
              <File size={16} />
              <span>{preview.path}</span>
              <small>{streamId === null ? "준비 중" : "tail -F 추적 중"}</small>
            </div>
            <button
              onClick={() => {
                if (streamId !== null)
                  void invoke("stop_tail_stream", { streamId });
                setStreamId(null);
                setPreview(null);
              }}
            >
              <X size={18} />
            </button>
          </header>
          <pre
            ref={previewScroll}
            onScroll={(event) => {
              const element = event.currentTarget;
              previewStickToBottom.current =
                element.scrollHeight -
                  element.scrollTop -
                  element.clientHeight <
                24;
            }}
          >
            <LogContent content={preview.content} />
          </pre>
        </section>
      )}
    </section>
  );
}
