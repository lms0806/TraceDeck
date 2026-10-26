import { FormEvent, useEffect, useState } from "react";
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
  const [tabs, setTabs] = useState<Tab[]>([{ id: "home", title: "Home" }]);
  const [activeId, setActiveId] = useState("home");
  const [selectedId, setSelectedId] = useState("");
  const [notice, setNotice] = useState(
    "서버를 선택하고 ‘폴더 열기’를 눌러 원격 파일을 확인하세요.",
  );
  const [dialog, setDialog] = useState(false);
  const [draft, setDraft] = useState(emptyProfile);
  useEffect(() => {
    invoke<Profile[]>("list_profiles")
      .then((saved) => saved.length && setProfiles(saved))
      .catch(() => undefined);
  }, []);
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
    <main className="app-shell">
      <header className="topbar">
        <div className="brand">
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
      <section className="workspace">
        <TabBar
          tabs={tabs}
          activeId={activeId}
          onActivate={setActiveId}
          onClose={closeSession}
          onMove={moveTab}
        />
        {activeId === "home" && <Home open={openProfile} profiles={profiles} />}
        {tabs
          .filter((tab) => tab.profileId)
          .flatMap((tab) => {
            const profile = profiles.find((item) => item.id === tab.profileId);
            if (!profile) return [];
            return (
              <div
                className={tab.id === activeId ? "tab-view" : "tab-view hidden"}
                key={tab.id}
              >
                <Browser
                  profile={profile}
                  active={tab.id === activeId}
                  onCheck={() => check(profile)}
                  notify={setNotice}
                />
              </div>
            );
          })}
        {tabs.some((tab) => tab.id === "analysis") && (
          <div
            className={activeId === "analysis" ? "tab-view" : "tab-view hidden"}
          >
            <Analysis profiles={profiles} notify={setNotice} />
          </div>
        )}
      </section>
      <div className="notice">{notice}</div>
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
    <section className="analysis-view">
      <header className="analysis-header">
        <div>
          <p>TRACING LOG ANALYSIS</p>
          <h1>원격 로그 분석</h1>
          <span>
            세션과 .log 파일을 선택하면 tracing의 레벨과 코드 모듈을 집계합니다.
          </span>
        </div>
      </header>
      <section className="analysis-controls">
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
        <label className="analysis-path">
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
      <div className="analysis-layout">
        <Panel className="analysis-files">
          <header>
            <h2>분석 대상 로그</h2>
            <span>{selected.length}개 선택</span>
          </header>
          {logFiles.length === 0 ? (
            <p className="analysis-empty">
              세션과 로그 폴더를 선택한 뒤 “파일 찾기”를 누르세요. 해당 폴더
              안의 .log 및 회전 로그 파일을 체크해 여러 개 선택할 수 있습니다.
            </p>
          ) : (
            logFiles.map((file) => (
              <label className="log-file-choice" key={file.path}>
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
            className="analyze-button"
            disabled={!profile || selected.length === 0 || scanning}
            onClick={() => void analyze()}
          >
            {scanning ? "분석 중…" : "선택 파일 분석"}
          </PrimaryButton>
          <small className="analysis-limit">
            각 파일의 최신 5,000줄만 읽습니다.
          </small>
        </Panel>
        <section className="analysis-results">
          <div className="metric-row">
            <article className="metric">
              <span>{selectedLevels.length ? "표시 로그" : "분석된 로그"}</span>
              <strong>{visibleRecords.length.toLocaleString()}</strong>
            </article>
            {Object.entries(levelCounts).map(([level, count]) => (
              <button
                className={
                  selectedLevels.includes(level)
                    ? `metric level-${level.toLowerCase()} selected-level`
                    : `metric level-${level.toLowerCase()}`
                }
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
            <div className="level-filter-note">
              <span>{selectedLevels.join(", ")} 로그만 표시 중</span>
              <button onClick={() => setSelectedLevels([])}>필터 해제</button>
            </div>
          )}
          <Panel className="source-table">
            <header>
              <h2>코드 위치별 로그</h2>
              <span>target / module path</span>
            </header>
            {sources.length === 0 ? (
              <p className="analysis-empty">분석 결과가 여기에 표시됩니다.</p>
            ) : (
              <div>
                {sources.map(([target, count]) => (
                  <div className="source-row" key={target}>
                    <code>{target}</code>
                    <b>{count.toLocaleString()}건</b>
                  </div>
                ))}
              </div>
            )}
          </Panel>
          {visibleRecords.length > 0 && (
            <Panel className="sample-table">
              <header>
                <h2>최근 로그 예시</h2>
              </header>
              {visibleRecords
                .slice(-20)
                .reverse()
                .map((record, index) => (
                  <div
                    className="sample-row"
                    key={`${record.timestamp}-${index}`}
                  >
                    <time>{record.timestamp}</time>
                    <em className={record.level.toLowerCase()}>
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
    <div className="modal-backdrop">
      <form className="session-dialog" onSubmit={submit}>
        <header>
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
        <div className="field-grid">
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
        <div className="key-picker">
          <span>{draft.privateKeyPath ?? "선택한 개인키가 없습니다."}</span>
          <button type="button" onClick={chooseKey}>
            <KeyRound size={14} /> 키 파일 선택
          </button>
        </div>
        <footer>
          <button type="button" onClick={close}>
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
