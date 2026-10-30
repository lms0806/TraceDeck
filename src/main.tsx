import type { FormEvent } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { CircleDot, FolderOpen, KeyRound, Plus, X } from "lucide-react";
import "./styles.css";
import { Browser } from "./components/Browser";
import { Sidebar } from "./components/app/Sidebar";
import { TabBar } from "./components/app/TabBar";
import { Panel } from "./components/ui/Panel";
import { IconButton } from "./components/ui/IconButton";
import { PrimaryButton } from "./components/ui/PrimaryButton";
import { parseTracing, type TracingRecord } from "./logging/tracing";
import {
  defaultWorkspace,
  loadWorkspace,
  saveWorkspace,
  type BrowserWorkspace,
} from "./app/workspace";

import type {
  ConnectionCheck,
  LogSource,
  Profile,
  RemoteEntry,
  Tab,
} from "./app/types";
const isLogFileName = (name: string) => /\.log(?:[._-].*)?$/i.test(name);
const emptyProfile = (): Profile => ({
  id: crypto.randomUUID(),
  name: "",
  host: "",
  port: 22,
  username: "",
  group: "Development",
  authKind: "privateKey",
});

function App() {
  const [profiles, setProfiles] = useState(initialProfiles);
  const [tabs, setTabs] = useState<Tab[]>(defaultWorkspace().tabs);
  const [activeId, setActiveId] = useState(defaultWorkspace().activeId);
  const [selectedId, setSelectedId] = useState(defaultWorkspace().selectedId);
  const [browserWorkspaces, setBrowserWorkspaces] = useState<
    Record<string, BrowserWorkspace>
  >({});
  const [profilesReady, setProfilesReady] = useState(false);
  const [workspaceReady, setWorkspaceReady] = useState(false);
  const workspaceRestored = useRef(false);
  const [notice, setNotice] = useState(
    "서버를 선택하고 ‘폴더 열기’를 눌러 원격 파일을 확인하세요.",
  );
  const [dialog, setDialog] = useState(false);
  const [draft, setDraft] = useState(emptyProfile);
  useEffect(() => {
    invoke<Profile[]>("list_profiles")
      .then((saved) => saved.length && setProfiles(saved))
      .catch(() => undefined)
      .finally(() => setProfilesReady(true));
  }, []);
  useEffect(() => {
    if (!profilesReady || workspaceRestored.current) return;
    workspaceRestored.current = true;
    void loadWorkspace()
      .then((workspace) => {
        const profileIds = new Set(profiles.map((profile) => profile.id));
        const restoredTabs = workspace.tabs.filter(
          (tab) => tab.profileId === undefined || profileIds.has(tab.profileId),
        );
        const restoredActiveId = restoredTabs.some(
          (tab) => tab.id === workspace.activeId,
        )
          ? workspace.activeId
          : "home";
        setTabs(restoredTabs);
        setActiveId(restoredActiveId);
        setSelectedId(workspace.selectedId);
        setBrowserWorkspaces(workspace.browsers);
      })
      .catch(() => undefined)
      .finally(() => setWorkspaceReady(true));
  }, [profiles, profilesReady]);
  useEffect(() => {
    if (!workspaceReady) return;
    void saveWorkspace({
      activeId,
      browsers: browserWorkspaces,
      selectedId,
      tabs,
    }).catch(() => undefined);
  }, [activeId, browserWorkspaces, selectedId, tabs, workspaceReady]);
  const updateBrowserWorkspace = useCallback(
    (profileId: string, workspace: BrowserWorkspace) => {
      setBrowserWorkspaces((current) => {
        const previous = current[profileId];
        if (
          previous.directoryPath === workspace.directoryPath &&
          previous.dockHeight === workspace.dockHeight &&
          previous.showTerminalDock === workspace.showTerminalDock &&
          previous.view === workspace.view
        )
          return current;
        return { ...current, [profileId]: workspace };
      });
    },
    [],
  );
  const active = tabs.find((tab) => tab.id === activeId) ?? tabs[0];
  function openProfile(profile: Profile) {
    setSelectedId(profile.id);
    const found = tabs.find((tab) => tab.profileId === profile.id);
    if (found) {
      setActiveId(found.id);
      return;
    }
    const id = `server-${profile.id}`;
    setTabs((all) => [
      ...all,
      { id, title: profile.name, profileId: profile.id },
    ]);
    setActiveId(id);
  }
  function closeSession(id: string) {
    setTabs((all) => all.filter((tab) => tab.id !== id));
    if (id === activeId) setActiveId("home");
  }
  function moveTab(draggedId: string, targetId: string, insertAfter: boolean) {
    if (draggedId === targetId || draggedId === "home") return;
    setTabs((current) => {
      const dragged = current.find((tab) => tab.id === draggedId);
      if (!dragged) return current;
      const withoutDragged = current.filter((tab) => tab.id !== draggedId);
      const targetIndex = withoutDragged.findIndex(
        (tab) => tab.id === targetId,
      );
      const insertAt =
        targetId === "home" ? 1 : targetIndex + Number(insertAfter);
      if (insertAt < 0) return current;
      return [
        ...withoutDragged.slice(0, insertAt),
        dragged,
        ...withoutDragged.slice(insertAt),
      ];
    });
  }
  function openAnalysis() {
    const found = tabs.find((tab) => tab.id === "analysis");
    if (!found) setTabs((all) => [...all, { id: "analysis", title: "분석" }]);
    setActiveId("analysis");
  }
  function removeProfile(profile: Profile) {
    if (!window.confirm(`저장된 세션 “${profile.name}”을 삭제할까요?`)) return;
    setProfiles((all) => all.filter((item) => item.id !== profile.id));
    setTabs((all) => all.filter((tab) => tab.profileId !== profile.id));
    setBrowserWorkspaces((all) => {
      const remaining = { ...all };
      delete remaining[profile.id];
      return remaining;
    });
    if (selectedId === profile.id) setSelectedId("");
    if (active.profileId === profile.id) setActiveId("home");
    invoke("delete_profile", { profileId: profile.id })
      .then(() => setNotice(`${profile.name} 세션을 삭제했습니다.`))
      .catch(() => setNotice("세션 삭제를 저장하지 못했습니다."));
  }
  async function check(profile: Profile) {
    setNotice("SSH 인증을 확인하는 중입니다…");
    try {
      const result = await invoke<ConnectionCheck>("check_ssh_connection", {
        profile,
      });
      setNotice(
        result.connected
          ? `연결 확인 완료: ${result.message}`
          : `연결 실패: ${result.message}`,
      );
    } catch (error) {
      setNotice(String(error));
    }
  }
  function save(event: FormEvent) {
    event.preventDefault();
    if (!draft.name || !draft.host || !draft.username) {
      setNotice("세션 이름, 호스트, 사용자 이름을 입력하세요.");
      return;
    }
    setProfiles((all) => [...all, draft]);
    invoke("save_profile", { profile: draft }).catch(() =>
      setNotice("프로필 저장에 실패했습니다."),
    );
    setDialog(false);
    openProfile(draft);
    setDraft(emptyProfile());
  }
  return (
    <main className="relative grid h-screen grid-cols-[280px_1fr] grid-rows-[52px_1fr] bg-[#111820]">
      <header className="col-span-2 flex items-center gap-[18px] border-b border-[#293846] bg-[#18232e] px-4">
        <div className="flex items-center gap-2 font-[750] text-white [&>svg]:text-[#50d3a1] [&>small]:ml-1 [&>small]:text-[10px] [&>small]:tracking-[1px] [&>small]:text-[#718397]">
          <CircleDot size={20} />
          <span>Tracing Log</span>
          <small>SFTP FILE BROWSER</small>
        </div>
        <PrimaryButton
          onClick={() => {
            setDraft(emptyProfile());
            setDialog(true);
          }}
        >
          <Plus size={16} /> 새 세션
        </PrimaryButton>
      </header>
      <Sidebar
        profiles={profiles}
        selectedId={selectedId}
        tabs={tabs}
        onOpenProfile={openProfile}
        onRemoveProfile={removeProfile}
        onCloseSession={closeSession}
        onNewSession={() => {
          setDraft(emptyProfile());
          setDialog(true);
        }}
        onHome={() => setActiveId("home")}
        onAnalysis={openAnalysis}
      />
      <section className="flex min-h-0 min-w-0 flex-col">
        <TabBar
          tabs={tabs}
          activeId={activeId}
          onActivate={setActiveId}
          onClose={closeSession}
          onMove={moveTab}
        />
        {activeId === "home" && <Home open={openProfile} profiles={profiles} />}
        {tabs
          .filter((tab) => tab.profileId !== undefined)
          .flatMap((tab) => {
            const profile = profiles.find((item) => item.id === tab.profileId);
            if (!profile) return [];
            return (
              <div
                className={
                  tab.id === activeId ? "flex min-h-0 flex-1" : "hidden"
                }
                key={tab.id}
              >
                <Browser
                  profile={profile}
                  active={tab.id === activeId}
                  onCheck={() => void check(profile)}
                  notify={setNotice}
                  workspace={browserWorkspaces[profile.id]}
                  onWorkspaceChange={(workspace) =>
                    updateBrowserWorkspace(profile.id, workspace)
                  }
                />
              </div>
            );
          })}
        {tabs.some((tab) => tab.id === "analysis") && (
          <div
            className={
              activeId === "analysis" ? "flex min-h-0 flex-1" : "hidden"
            }
          >
            <Analysis profiles={profiles} notify={setNotice} />
          </div>
        )}
      </section>
      <div className="absolute right-[18px] top-[67px] z-[5] max-w-[470px] rounded-md border border-[#3b5164] bg-[#1d2a36] px-[13px] py-2.5 text-xs text-[#bdcdda] shadow-[0_8px_22px_#0007]">
        {notice}
      </div>
      {dialog && (
        <SessionDialog
          draft={draft}
          setDraft={setDraft}
          close={() => setDialog(false)}
          submit={save}
        />
      )}
    </main>
  );
}

function Home({
  profiles,
  open,
}: {
  profiles: Profile[];
  open: (profile: Profile) => void;
}) {
  return (
    <section className="overflow-auto p-[34px]">
      <div className="flex justify-between gap-[30px] rounded-[10px] border border-[#355365] bg-[linear-gradient(120deg,#1e3f46,#192c39_65%)] p-7">
        <div>
          <p className="mb-[6px] text-[10px] font-extrabold tracking-[1.2px] text-[#59d6a5]">
            REMOTE FILE WORKSPACE
          </p>
          <h1 className="mb-2.5 max-w-[570px] text-[25px] leading-[1.35] text-[#f4fbff]">
            원격 서버 파일을 GUI로 관리하세요.
          </h1>
          <span className="text-[13px] text-[#a8bbc9]">
            서버를 선택한 뒤 SFTP 폴더 브라우저를 엽니다.
          </span>
        </div>
      </div>
      <Panel as="div" className="mt-[18px] quick-list">
        <header>
          <h2>빠른 연결</h2>
        </header>
        {profiles.map((profile) => (
          <button
            className="flex w-full items-center gap-2.5 border-0 border-b border-[#293947] bg-transparent px-0.5 py-[11px] text-left last:border-0 [&>svg]:text-[#75a4c5] [&>span]:grid [&>span]:gap-[3px] [&_strong]:text-xs [&_small]:text-[11px] [&_small]:text-[#8196a8]"
            key={profile.id}
            onClick={() => open(profile)}
          >
            <FolderOpen size={18} />
            <span>
              <strong>{profile.name}</strong>
              <small>
                {profile.username}@{profile.host}:{profile.port}
              </small>
            </span>
          </button>
        ))}
      </Panel>
    </section>
  );
}

function Analysis({
  profiles,
  notify,
}: {
  profiles: Profile[];
  notify: (text: string) => void;
}) {
  const [profileId, setProfileId] = useState("");
  const [path, setPath] = useState(".");
  const [entries, setEntries] = useState<RemoteEntry[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [records, setRecords] = useState<TracingRecord[]>([]);
  const [selectedLevels, setSelectedLevels] = useState<string[]>([]);
  const [scanning, setScanning] = useState(false);
  const profile = profiles.find((item) => item.id === profileId);
  const logFiles = entries.filter(
    (entry) => !entry.isDir && isLogFileName(entry.name),
  );
  const levelCounts = records.reduce<Record<string, number>>(
    (counts, record) => {
      counts[record.level] = (counts[record.level] ?? 0) + 1;
      return counts;
    },
    {},
  );
  const visibleRecords = selectedLevels.length
    ? records.filter((record) => selectedLevels.includes(record.level))
    : records;
  const targetCounts = visibleRecords.reduce<Record<string, number>>(
    (counts, record) => {
      counts[record.target] = (counts[record.target] ?? 0) + 1;
      return counts;
    },
    {},
  );
  const sources = Object.entries(targetCounts).sort(([, a], [, b]) => b - a);

  async function loadFiles() {
    if (!profile) return;
    setScanning(true);
    try {
      const result = await invoke<RemoteEntry[]>("list_remote_directory", {
        profile,
        path,
      });
      setEntries(result);
      setSelected((current) =>
        current.filter((item) => result.some((entry) => entry.path === item)),
      );
      notify(
        `${result.filter((entry) => !entry.isDir && isLogFileName(entry.name)).length}개의 로그 파일을 찾았습니다.`,
      );
    } catch (error) {
      notify(`로그 목록을 읽지 못했습니다: ${String(error)}`);
    } finally {
      setScanning(false);
    }
  }

  async function analyze() {
    if (!profile || selected.length === 0) return;
    setScanning(true);
    try {
      const files = await invoke<LogSource[]>("read_remote_log_files", {
        profile,
        paths: selected,
      });
      const parsed = files.flatMap((file) => parseTracing(file.content));
      setRecords(parsed);
      setSelectedLevels([]);
      notify(
        `${files.length}개 파일에서 tracing 로그 ${parsed.length}건을 분석했습니다.`,
      );
    } catch (error) {
      notify(`로그 분석에 실패했습니다: ${String(error)}`);
    } finally {
      setScanning(false);
    }
  }

  return (
    <section className="min-h-0 overflow-auto px-[30px] py-[26px]">
      <header className="rounded-[9px] border border-[#355365] bg-[linear-gradient(120deg,#1e3f46,#192c39_65%)] px-6 py-[22px] [&_h1]:mb-2 [&_h1]:text-[23px] [&_h1]:text-white [&_p]:mb-[6px] [&_p]:text-[10px] [&_p]:font-extrabold [&_p]:tracking-[1.2px] [&_p]:text-[#59d6a5] [&_span]:text-xs [&_span]:text-[#a8bbc9]">
        <div>
          <p>TRACING LOG ANALYSIS</p>
          <h1>원격 로그 분석</h1>
          <span>
            세션과 .log 파일을 선택하면 tracing의 레벨과 코드 모듈을 집계합니다.
          </span>
        </div>
      </header>
      <section className="my-4 flex items-end gap-2.5 rounded-lg border border-[#2d3d4c] bg-[#17212b] p-[14px] [&_input]:w-full [&_input]:rounded-[5px] [&_input]:border [&_input]:border-[#3c5062] [&_input]:bg-[#101820] [&_input]:p-2 [&_input]:text-[#e6eef4] [&_label]:grid [&_label]:min-w-[230px] [&_label]:gap-1.5 [&_label]:text-[11px] [&_label]:text-[#9eb1c1] [&_select]:w-full [&_select]:rounded-[5px] [&_select]:border [&_select]:border-[#3c5062] [&_select]:bg-[#101820] [&_select]:p-2 [&_select]:text-[#e6eef4]">
        <label>
          세션
          <select
            value={profileId}
            onChange={(event) => {
              setProfileId(event.target.value);
              setEntries([]);
              setSelected([]);
              setRecords([]);
            }}
          >
            <option value="">세션 선택</option>
            {profiles.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name} — {item.username}@{item.host}
              </option>
            ))}
          </select>
        </label>
        <label className="flex-1">
          로그 폴더
          <input
            value={path}
            onChange={(event) => setPath(event.target.value)}
            onKeyDown={(event) => event.key === "Enter" && void loadFiles()}
            placeholder="예: /var/log/service"
          />
        </label>
        <PrimaryButton
          disabled={!profile || scanning}
          onClick={() => void loadFiles()}
        >
          파일 찾기
        </PrimaryButton>
      </section>
      <div className="grid grid-cols-[minmax(260px,0.65fr)_minmax(450px,1.35fr)] gap-4">
        <Panel className="self-start [&>header]:items-center">
          <header>
            <h2>분석 대상 로그</h2>
            <span>{selected.length}개 선택</span>
          </header>
          {logFiles.length === 0 ? (
            <p className="text-xs leading-[1.55] text-[#7e96a8]">
              세션과 로그 폴더를 선택한 뒤 “파일 찾기”를 누르세요. 해당 폴더
              안의 .log 및 회전 로그 파일을 체크해 여러 개 선택할 수 있습니다.
            </p>
          ) : (
            logFiles.map((file) => (
              <label
                className="flex cursor-pointer gap-[9px] border-b border-[#293947] px-[3px] py-2.5 [&>input]:accent-[#50d3a1] [&>span]:grid [&>span]:min-w-0 [&>span]:gap-[3px] [&_b]:overflow-hidden [&_b]:text-xs [&_b]:text-ellipsis [&_small]:overflow-hidden [&_small]:whitespace-nowrap [&_small]:text-ellipsis [&_small]:font-mono [&_small]:text-[10px] [&_small]:text-[#7c94a6]"
                key={file.path}
              >
                <input
                  type="checkbox"
                  checked={selected.includes(file.path)}
                  onChange={(event) =>
                    setSelected((items) =>
                      event.target.checked
                        ? [...items, file.path]
                        : items.filter((item) => item !== file.path),
                    )
                  }
                />
                <span>
                  <b>{file.name}</b>
                  <small>{file.details}</small>
                </span>
              </label>
            ))
          )}
          <PrimaryButton
            className="mt-[14px]"
            disabled={!profile || selected.length === 0 || scanning}
            onClick={() => void analyze()}
          >
            {scanning ? "분석 중…" : "선택 파일 분석"}
          </PrimaryButton>
          <small className="mt-[9px] block text-[10px] text-[#72899b]">
            각 파일의 최신 5,000줄만 읽습니다.
          </small>
        </Panel>
        <section className="analysis-results">
          <div className="mb-4 flex flex-wrap gap-2.5">
            <article className="min-w-[110px] rounded-lg border border-[#2d3d4c] bg-[#17212b] p-[13px] [&>span]:block [&>span]:text-[10px] [&>span]:font-bold [&>span]:text-[#90a4b5] [&>strong]:mt-[5px] [&>strong]:block [&>strong]:text-[21px] [&>strong]:text-[#e7f4fb]">
              <span>{selectedLevels.length ? "표시 로그" : "분석된 로그"}</span>
              <strong>{visibleRecords.length.toLocaleString()}</strong>
            </article>
            {Object.entries(levelCounts).map(([level, count]) => (
              <button
                className={`min-w-[110px] rounded-lg border p-[13px] text-left transition-colors hover:bg-[#21303c] [&>span]:block [&>span]:text-[10px] [&>span]:font-bold [&>span]:text-[#90a4b5] [&>strong]:mt-[5px] [&>strong]:block [&>strong]:text-[21px] ${level === "ERROR" ? "border-[#6a343b] [&>strong]:text-[#fa9893]" : level === "WARN" ? "border-[#705621] [&>strong]:text-[#e7b65e]" : "border-[#2b6251] [&>strong]:text-[#6fd1aa]"} ${selectedLevels.includes(level) ? "bg-[#1a3b35] shadow-[inset_0_0_0_1px_#58d6a540]" : "bg-[#17212b]"}`}
                key={level}
                aria-pressed={selectedLevels.includes(level)}
                onClick={() =>
                  setSelectedLevels((levels) =>
                    levels.includes(level)
                      ? levels.filter((item) => item !== level)
                      : [...levels, level],
                  )
                }
              >
                <span>{level}</span>
                <strong>{count.toLocaleString()}</strong>
              </button>
            ))}
          </div>
          {selectedLevels.length > 0 && (
            <div className="-mt-[7px] mb-4 flex items-center justify-between text-[11px] text-[#90bfae]">
              <span>{selectedLevels.join(", ")} 로그만 표시 중</span>
              <button
                className="text-[11px] text-[#78dba9] underline"
                onClick={() => setSelectedLevels([])}
              >
                필터 해제
              </button>
            </div>
          )}
          <Panel className="mb-4 [&>header]:items-center">
            <header>
              <h2>코드 위치별 로그</h2>
              <span>target / module path</span>
            </header>
            {sources.length === 0 ? (
              <p className="text-xs leading-[1.55] text-[#7e96a8]">
                분석 결과가 여기에 표시됩니다.
              </p>
            ) : (
              <div>
                {sources.map(([target, count]) => (
                  <div
                    className="flex justify-between gap-3 border-b border-[#293947] py-2.5 last:border-0"
                    key={target}
                  >
                    <code>{target}</code>
                    <b>{count.toLocaleString()}건</b>
                  </div>
                ))}
              </div>
            )}
          </Panel>
          {visibleRecords.length > 0 && (
            <Panel className="mb-4 [&>header]:items-center">
              <header>
                <h2>최근 로그 예시</h2>
              </header>
              {visibleRecords
                .slice(-20)
                .reverse()
                .map((record, index) => (
                  <div
                    className="grid grid-cols-[170px_52px_minmax(170px,0.7fr)_1.3fr] items-center gap-[9px] border-b border-[#293947] py-2 font-mono text-[10px] [&>code]:overflow-hidden [&>code]:whitespace-nowrap [&>code]:text-ellipsis [&>code]:text-[#a9c9dd] [&>span]:overflow-hidden [&>span]:whitespace-nowrap [&>span]:text-ellipsis [&>span]:text-[#c5d4df] [&>time]:text-[#7894aa]"
                    key={`${record.timestamp}-${index}`}
                  >
                    <time>{record.timestamp}</time>
                    <em
                      className={`w-max rounded-[3px] px-1 py-0.5 not-italic font-bold ${record.level === "ERROR" ? "bg-[#4c2529] text-[#fa9893]" : record.level === "WARN" ? "bg-[#493716] text-[#e7b65e]" : "bg-[#153d32] text-[#6fd1aa]"}`}
                    >
                      {record.level}
                    </em>
                    <code>{record.target}</code>
                    <span>{record.message}</span>
                  </div>
                ))}
            </Panel>
          )}
        </section>
      </div>
    </section>
  );
}

function SessionDialog({
  draft,
  setDraft,
  close,
  submit,
}: {
  draft: Profile;
  setDraft: (profile: Profile) => void;
  close: () => void;
  submit: (event: FormEvent) => void;
}) {
  const update = (field: keyof Profile, value: string | number) =>
    setDraft({ ...draft, [field]: value });
  async function chooseKey() {
    const file = await open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: "OpenSSH private key",
          extensions: ["pem", "key", "rsa", "ed25519"],
        },
      ],
    });
    if (typeof file === "string")
      setDraft({ ...draft, privateKeyPath: file, authKind: "privateKey" });
  }
  return (
    <div className="absolute inset-0 z-10 grid place-items-center bg-[#070b0fc9]">
      <form
        className="w-[490px] rounded-[9px] border border-[#405565] bg-[#17222c] p-5 shadow-[0_20px_70px_#000] [&>label]:mt-[14px] [&>label]:grid [&>label]:gap-1.5 [&>label]:text-[11px] [&>label]:text-[#9eb1c1] [&_input]:w-full [&_input]:rounded-[5px] [&_input]:border [&_input]:border-[#3c5062] [&_input]:bg-[#101820] [&_input]:p-[9px] [&_input]:text-xs [&_input]:text-[#e6eef4] [&_input:focus]:border-[#55cda0]"
        onSubmit={submit}
      >
        <header className="mb-4 flex items-start justify-between [&_h2]:m-0 [&_h2]:text-lg [&_h2]:text-[#eff7fc]">
          <div>
            <p className="mb-[6px] text-[10px] font-extrabold tracking-[1.2px] text-[#59d6a5]">
              NEW SFTP SESSION
            </p>
            <h2>원격 서버 추가</h2>
          </div>
          <IconButton
            className="text-[#91a5b6] hover:bg-transparent"
            onClick={close}
          >
            <X size={18} />
          </IconButton>
        </header>
        <label>
          세션 이름
          <input
            autoFocus
            value={draft.name}
            onChange={(e) => update("name", e.target.value)}
          />
        </label>
        <div className="grid grid-cols-[1fr_110px] gap-3 [&_label]:mt-[14px] [&_label]:grid [&_label]:gap-1.5 [&_label]:text-[11px] [&_label]:text-[#9eb1c1]">
          <label>
            호스트
            <input
              value={draft.host}
              onChange={(e) => update("host", e.target.value)}
            />
          </label>
          <label>
            포트
            <input
              type="number"
              value={draft.port}
              onChange={(e) => update("port", Number(e.target.value))}
            />
          </label>
        </div>
        <label>
          사용자 이름
          <input
            value={draft.username}
            onChange={(e) => update("username", e.target.value)}
          />
        </label>
        <div className="mt-4 flex min-w-0 items-center gap-2 rounded-md border border-[#3c5062] bg-[#101820] p-2">
          <span className="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap px-1 text-xs text-[#9eb1c1]">
            {draft.privateKeyPath ?? "선택한 개인키가 없습니다."}
          </span>
          <button
            className="flex flex-none items-center gap-1.5 rounded border border-[#3c695f] bg-[#17352e] px-2.5 py-1.5 text-[11px] font-bold text-[#78dfb5] hover:bg-[#21483e]"
            type="button"
            onClick={() => void chooseKey()}
          >
            <KeyRound size={14} /> 키 파일 선택
          </button>
        </div>
        <footer className="mt-5 flex justify-end gap-2">
          <button
            className="border-0 bg-transparent px-[11px] py-2 text-[#9aafbf]"
            type="button"
            onClick={close}
          >
            취소
          </button>
          <PrimaryButton type="submit">저장하고 열기</PrimaryButton>
        </footer>
      </form>
    </div>
  );
}
const root = document.getElementById("root");
if (!root) throw new Error("#root를 찾지 못했습니다.");
createRoot(root).render(<App />);
