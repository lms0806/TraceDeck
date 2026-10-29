import {
  File,
  Folder,
  FolderOpen,
  FolderTree,
  FolderUp,
  HardDrive,
} from "lucide-react";
import { useState } from "react";

import type { RemoteEntry } from "./types";

export function FileList({
  entries,
  loaded,
  loading,
  onOpenDirectory,
  onOpenParent,
  canOpenParent,
  onOpenFile,
  onDeleteEntry,
  onRenameEntry,
  onCreateFile,
  onCreateDirectory,
}: {
  entries: RemoteEntry[];
  loaded: boolean;
  loading: boolean;
  onOpenDirectory: (path: string) => void;
  onOpenParent: () => void;
  canOpenParent: boolean;
  onOpenFile: (entry: RemoteEntry) => void;
  onDeleteEntry: (entry: RemoteEntry) => void;
  onRenameEntry: (entry: RemoteEntry) => void;
  onCreateFile: () => void;
  onCreateDirectory: () => void;
}) {
  const [contextMenu, setContextMenu] = useState<{
    entry?: RemoteEntry;
    x: number;
    y: number;
  } | null>(null);

  return (
    <div
      className="grid min-h-0 flex-1 grid-cols-[220px_1fr]"
      onClick={() => setContextMenu(null)}
    >
      <aside className="border-r border-[#293846] bg-[#151e28] p-[13px]">
        <header className="mb-2.5 flex items-center gap-[7px] text-xs text-[#a9bdcc]">
          <FolderTree size={16} /> 위치
        </header>
        <button
          className="flex w-full gap-[7px] rounded bg-transparent p-2 text-left text-xs text-[#b8c8d4] hover:bg-[#243341]"
          onClick={() => onOpenDirectory(".")}
        >
          <Folder size={15} /> 홈
        </button>
        <button
          className="flex w-full gap-[7px] rounded bg-transparent p-2 text-left text-xs text-[#b8c8d4] hover:bg-[#243341]"
          onClick={() => onOpenDirectory("/")}
        >
          <HardDrive size={15} /> /
        </button>
        <p className="mx-[6px] my-[18px] text-[10px] leading-normal text-[#72899b]">
          목록은 변경된 경우에만 갱신됩니다.
        </p>
      </aside>
      <section
        className="min-w-0 overflow-auto bg-[#111920]"
        onContextMenu={(event) => {
          if ((event.target as HTMLElement).closest("[data-entry]")) return;
          event.preventDefault();
          setContextMenu({ x: event.clientX, y: event.clientY });
        }}
      >
        <header className="border-b border-[#293846] px-[15px] py-2.5 text-[10px] tracking-[.7px] text-[#71879a]">
          <div className="grid grid-cols-[minmax(240px,1fr)_minmax(280px,1.4fr)] gap-[15px]">
            <span>이름</span>
            <span>권한 · 소유자 · 크기</span>
          </div>
        </header>
        {!loaded && (
          <div className="grid min-h-[250px] place-content-center justify-items-center gap-[9px] p-[25px] text-center text-xs text-[#7e96a8] [&>strong]:text-[13px] [&>strong]:text-[#b9cad6] [&>svg]:text-[#5f8099]">
            <FolderOpen size={34} />
            <strong>원격 폴더를 열 준비가 됐습니다.</strong>
            <span>“폴더 열기”를 누르면 서버의 파일 목록을 가져옵니다.</span>
          </div>
        )}
        {loaded && (
          <button
            className="grid w-full grid-cols-[minmax(240px,1fr)_minmax(280px,1.4fr)] gap-[15px] border-0 border-b border-[#1e2b36] bg-transparent px-[15px] py-[9px] text-left text-[#cbd8e2] hover:bg-[#1b2a35] disabled:cursor-not-allowed disabled:opacity-45 [&>span]:flex [&>span]:items-center [&>span]:gap-2 [&_b]:text-xs [&_b]:font-[550]"
            disabled={loading || !canOpenParent}
            onClick={onOpenParent}
            title="상위 폴더"
          >
            <span>
              <FolderUp className="text-[#e7bc66]" size={17} />
              <b>..</b>
            </span>
            <small className="font-mono text-[10px] text-[#7c94a6]">
              상위 폴더
            </small>
          </button>
        )}
        {loaded && !loading && entries.length === 0 && (
          <div className="grid min-h-[250px] place-content-center justify-items-center gap-[9px] p-[25px] text-center text-xs text-[#7e96a8] [&>strong]:text-[13px] [&>strong]:text-[#b9cad6] [&>svg]:text-[#5f8099]">
            <FolderOpen size={34} />
            <strong>폴더가 비어 있거나 읽을 수 없습니다.</strong>
          </div>
        )}
        {entries.map((entry) => (
          <button
            className="grid w-full grid-cols-[minmax(240px,1fr)_minmax(280px,1.4fr)] gap-[15px] border-0 border-b border-[#1e2b36] bg-transparent px-[15px] py-[9px] text-left text-[#cbd8e2] hover:bg-[#1b2a35] [&>span]:flex [&>span]:items-center [&>span]:gap-2 [&_b]:text-xs [&_b]:font-[550] [&_small]:overflow-hidden [&_small]:text-ellipsis [&_small]:whitespace-nowrap [&_small]:font-mono [&_small]:text-[10px] [&_small]:text-[#7c94a6]"
            key={entry.path}
            data-entry
            onClick={() =>
              entry.isDir ? onOpenDirectory(entry.path) : onOpenFile(entry)
            }
            onContextMenu={(event) => {
              event.preventDefault();
              setContextMenu({
                entry,
                x: event.clientX,
                y: event.clientY,
              });
            }}
            title={entry.details}
          >
            <span>
              {entry.isDir ? (
                <Folder className="text-[#e7bc66]" size={17} />
              ) : (
                <File size={17} />
              )}
              <b>{entry.name}</b>
            </span>
            <small>{entry.details}</small>
          </button>
        ))}
      </section>
      {contextMenu && (
        <div
          className="fixed z-20 min-w-[132px] rounded-md border border-[#405565] bg-[#1a2631] p-1 shadow-[0_10px_25px_#0008]"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onClick={(event) => event.stopPropagation()}
        >
          {contextMenu.entry ? (
            <>
              <button
                className="w-full rounded px-2.5 py-2 text-left text-xs text-[#d6e4ed] hover:bg-[#294052]"
                onClick={() => {
                  onRenameEntry(contextMenu.entry!);
                  setContextMenu(null);
                }}
              >
                이름 변경
              </button>
              <button
                className="w-full rounded px-2.5 py-2 text-left text-xs text-[#ffaaa5] hover:bg-[#41272a]"
                onClick={() => {
                  onDeleteEntry(contextMenu.entry!);
                  setContextMenu(null);
                }}
              >
                삭제
              </button>
            </>
          ) : (
            <>
              <button
                className="w-full rounded px-2.5 py-2 text-left text-xs text-[#d6e4ed] hover:bg-[#294052]"
                onClick={() => {
                  onCreateFile();
                  setContextMenu(null);
                }}
              >
                새 파일
              </button>
              <button
                className="w-full rounded px-2.5 py-2 text-left text-xs text-[#d6e4ed] hover:bg-[#294052]"
                onClick={() => {
                  onCreateDirectory();
                  setContextMenu(null);
                }}
              >
                새 폴더
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}
