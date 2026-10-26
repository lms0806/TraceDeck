import { File, Folder, FolderOpen, FolderTree, HardDrive } from "lucide-react";

import type { RemoteEntry } from "./types";

export function FileList({
  entries,
  loaded,
  loading,
  onOpenDirectory,
  onOpenFile,
}: {
  entries: RemoteEntry[];
  loaded: boolean;
  loading: boolean;
  onOpenDirectory: (path: string) => void;
  onOpenFile: (entry: RemoteEntry) => void;
}) {
  return (
    <div className="file-content">
      <aside className="folder-sidebar">
        <header>
          <FolderTree size={16} /> 위치
        </header>
        <button onClick={() => onOpenDirectory(".")}>
          <Folder size={15} /> 홈
        </button>
        <button onClick={() => onOpenDirectory("/")}>
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
              entry.isDir ? onOpenDirectory(entry.path) : onOpenFile(entry)
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
  );
}
