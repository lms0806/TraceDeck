import { Terminal } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { FileList } from "./browser/FileList";
import { LogDock } from "./browser/LogDock";
import { RemoteStatus } from "./browser/RemoteStatus";
import { TerminalDock } from "./browser/TerminalDock";
import type { Profile } from "./browser/types";
import type { BrowserWorkspace } from "../app/workspace";
import { useRemoteDirectory } from "./browser/useRemoteDirectory";
import { useTailStream } from "./browser/useTailStream";
import { useTerminal } from "./browser/useTerminal";
import { PrimaryButton } from "./ui/PrimaryButton";
import { SecondaryButton } from "./ui/SecondaryButton";

export function Browser({
  profile,
  active,
  onCheck,
  notify,
  workspace,
  onWorkspaceChange,
}: {
  profile: Profile;
  active: boolean;
  onCheck: () => void;
  notify: (text: string) => void;
  workspace?: BrowserWorkspace;
  onWorkspaceChange: (workspace: BrowserWorkspace) => void;
}) {
  const [view, setView] = useState<"files" | "terminal">(
    workspace?.view ?? "files",
  );
  const [showTerminalDock, setShowTerminalDock] = useState(
    workspace?.showTerminalDock ?? false,
  );
  const [dockHeight, setDockHeight] = useState(workspace?.dockHeight ?? 310);
  const directory = useRemoteDirectory(
    profile,
    active,
    notify,
    workspace?.directoryPath,
  );
  const restoredDirectory = useRef(false);
  const tail = useTailStream(profile);
  const terminal = useTerminal(
    profile,
    active && (view === "terminal" || showTerminalDock),
  );

  useEffect(() => {
    onWorkspaceChange({
      directoryPath: directory.path,
      dockHeight,
      showTerminalDock,
      view,
    });
  }, [directory.path, dockHeight, onWorkspaceChange, showTerminalDock, view]);

  useEffect(() => {
    if (!active || restoredDirectory.current || !workspace?.directoryPath)
      return;
    restoredDirectory.current = true;
    void directory.load(workspace.directoryPath, false, true);
    // Restore the previously-open directory once; subsequent refreshes are owned by the hook.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, workspace?.directoryPath]);

  async function deleteEntry(entry: Parameters<typeof tail.openFile>[0]) {
    const kind = entry.isDir ? "빈 폴더" : "파일";
    if (
      !window.confirm(
        `“${entry.name}” ${kind}을 삭제할까요? 이 작업은 되돌릴 수 없습니다.`,
      )
    )
      return;
    try {
      await invoke("delete_remote_entry", {
        profile,
        path: entry.path,
        isDirectory: entry.isDir,
      });
      notify(`${entry.name}을(를) 삭제했습니다.`);
      await directory.refresh();
    } catch (error) {
      notify(`삭제하지 못했습니다: ${String(error)}`);
    }
  }

  async function renameEntry(entry: Parameters<typeof tail.openFile>[0]) {
    const newName = window.prompt("새 이름을 입력하세요.", entry.name)?.trim();
    if (!newName || newName === entry.name) return;
    try {
      await invoke("rename_remote_entry", {
        profile,
        path: entry.path,
        newName,
      });
      notify(`${entry.name}의 이름을 ${newName}(으)로 변경했습니다.`);
      await directory.refresh();
    } catch (error) {
      notify(`이름을 변경하지 못했습니다: ${String(error)}`);
    }
  }

  async function createEntry(kind: "file" | "directory") {
    const label = kind === "file" ? "파일" : "폴더";
    const name = window.prompt(`새 ${label} 이름을 입력하세요.`)?.trim();
    if (!name) return;
    try {
      await invoke(
        kind === "file" ? "create_remote_file" : "create_remote_directory",
        { profile, parentPath: directory.path, name },
      );
      notify(`${name} ${label}을(를) 생성했습니다.`);
      await directory.refresh();
    } catch (error) {
      notify(`${label}을(를) 생성하지 못했습니다: ${String(error)}`);
    }
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <header className="flex h-12 items-center justify-between border-b border-[#293846] px-[18px] text-xs text-[#9fb2c2]">
        <div>
          <span className="mr-1.5 inline-block h-[7px] w-[7px] rounded-full bg-[#e5b662]" />{" "}
          {profile.username}@{profile.host}:{profile.port}
        </div>
        <div className="flex items-center gap-2">
          <button
            className={`rounded px-[9px] py-[7px] ${view === "files" ? "bg-[#263b4b] text-white" : "text-[#9fb2c2] hover:bg-[#263746]"}`}
            onClick={() => setView("files")}
          >
            파일
          </button>
          <button
            className={`flex items-center gap-1 rounded px-[9px] py-[7px] ${view === "terminal" ? "bg-[#263b4b] text-white" : "text-[#9fb2c2] hover:bg-[#263746]"}`}
            onClick={() => setView("terminal")}
          >
            <Terminal size={14} /> 터미널
          </button>
          <button
            className="rounded bg-[#236d56] px-[9px] py-[7px] text-[#effff8] hover:bg-[#359372]"
            onClick={onCheck}
          >
            연결 확인
          </button>
        </div>
      </header>
      {view === "terminal" ? (
        <TerminalDock
          terminal={terminal}
          expanded
          onClose={() => setView("files")}
        />
      ) : (
        <>
          <div className="flex items-center gap-2 border-b border-[#293846] bg-[#18232e] px-[14px] py-2.5">
            <SecondaryButton
              onClick={() => void directory.refresh()}
              disabled={directory.loading}
            >
              새로고침
            </SecondaryButton>
            <label className="flex flex-1 items-center gap-2 text-[11px] text-[#8ba1b4]">
              <span>원격 경로</span>
              <input
                className="min-w-0 flex-1 rounded border border-[#3a5061] bg-[#101820] px-[9px] py-[7px] text-[#e8f0f5]"
                value={directory.path}
                onChange={(event) => directory.setPath(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter")
                    void directory.load(directory.path, true);
                }}
              />
            </label>
            <PrimaryButton
              onClick={() => void directory.load(directory.path, true)}
              disabled={directory.loading}
            >
              {directory.loading ? "불러오는 중…" : "폴더 열기"}
            </PrimaryButton>
            <SecondaryButton onClick={() => setShowTerminalDock(true)}>
              <Terminal size={15} /> 하단 터미널
            </SecondaryButton>
          </div>
          <FileList
            entries={directory.entries}
            loaded={directory.loaded}
            loading={directory.loading}
            onOpenDirectory={(path) => void directory.load(path, true)}
            onOpenParent={directory.parent}
            canOpenParent={directory.canOpenParent}
            onOpenFile={(entry) => {
              setShowTerminalDock(false);
              void tail.openFile(entry);
            }}
            onDeleteEntry={(entry) => void deleteEntry(entry)}
            onRenameEntry={(entry) => void renameEntry(entry)}
            onCreateFile={() => void createEntry("file")}
            onCreateDirectory={() => void createEntry("directory")}
          />
          {showTerminalDock ? (
            <TerminalDock
              terminal={terminal}
              onClose={() => setShowTerminalDock(false)}
              logPath={tail.preview?.path}
              height={dockHeight}
              onHeightChange={setDockHeight}
              onShowLog={
                tail.preview ? () => setShowTerminalDock(false) : undefined
              }
            />
          ) : tail.preview ? (
            <LogDock
              preview={tail.preview}
              streamId={tail.streamId}
              onClose={tail.close}
              onOpenTerminal={() => setShowTerminalDock(true)}
              height={dockHeight}
              onHeightChange={setDockHeight}
            />
          ) : null}
        </>
      )}
      <RemoteStatus profile={profile} active={active} />
    </section>
  );
}
