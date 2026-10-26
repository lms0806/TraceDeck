import { FormEvent, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ChartNoAxesCombined,
  CircleDot,
  FolderOpen,
  KeyRound,
  MonitorCog,
  Plus,
  Search,
  Server,
  ShieldCheck,
  X,
} from "lucide-react";
import "./styles.css";
import { Browser } from "./components/Browser";

type AuthKind = "password" | "privateKey" | "agent";
type Profile = {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  group: string;
  authKind?: AuthKind;
  privateKeyPath?: string;
};
type Tab = { id: string; title: string; profileId?: string };
type ConnectionCheck = { connected: boolean; message: string };
type RemoteEntry = {
  name: string;
  path: string;
  isDir: boolean;
  details: string;
};
type LogSource = { path: string; content: string };
type TracingRecord = {
  timestamp: string;
  level: string;
  target: string;
  message: string;
};
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
  const [query, setQuery] = useState("");
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
  const shown = profiles.filter((profile) =>
    `${profile.name} ${profile.host} ${profile.group}`
      .toLowerCase()
      .includes(query.toLowerCase()),
  );
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
        <button
          className="primary"
          onClick={() => {
            setDraft(emptyProfile());
            setDialog(true);
          }}
        >
          <Plus size={16} /> 새 세션
        </button>
      </header>
      <aside className="sidebar">
        <label className="search-box">
          <Search size={15} />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="서버 검색"
          />
        </label>
        <div className="side-heading">
          <span>SAVED SESSIONS</span>
          <button onClick={() => setDialog(true)}>
            <Plus size={15} />
          </button>
        </div>
        <div className="session-tree">
          {shown.map((profile) => (
            <div className="saved-session" key={profile.id}>
              <button
                className={
                  selectedId === profile.id
                    ? "server-row selected"
                    : "server-row"
                }
                onClick={() => openProfile(profile)}
              >
                <span className="server-state" />
                <Server size={15} />
                <span>
                  <strong>{profile.name}</strong>
                  <small>
                    {profile.username}@{profile.host}
                  </small>
                </span>
                <span className="ssh-tag">SFTP</span>
              </button>
              <button
                className="delete-saved-session"
                onClick={() => removeProfile(profile)}
                aria-label={`${profile.name} 저장 세션 삭제`}
              >
                <X size={14} />
              </button>
            </div>
          ))}
        </div>
        <div className="side-heading open-heading">
          <span>OPEN SESSIONS</span>
        </div>
        <div className="open-sessions">
          {tabs
            .filter((tab) => tab.profileId)
            .map((tab) => (
              <div key={tab.id}>
                <button onClick={() => setActiveId(tab.id)}>
                  <FolderOpen size={14} />
                  {tab.title}
                </button>
                <button
                  className="close-session"
                  onClick={() => closeSession(tab.id)}
                  aria-label={`${tab.title} 세션 닫기`}
                >
                  <X size={14} />
                </button>
              </div>
            ))}
        </div>
        <div className="sidebar-footer">
          <button onClick={() => setActiveId("home")}>
            <MonitorCog size={16} /> 홈
          </button>
          <button onClick={openAnalysis}>
            <ChartNoAxesCombined size={16} /> 분석
          </button>
          <button>
            <ShieldCheck size={16} /> 보안 상태
          </button>
        </div>
      </aside>
      <section className="workspace">
        <nav className="tabs">
          {tabs.map((tab) => (
            <button
              className={tab.id === activeId ? "tab active" : "tab"}
              key={tab.id}
              onClick={() => setActiveId(tab.id)}
            >
              {tab.profileId ? (
                <FolderOpen size={15} />
              ) : tab.id === "analysis" ? (
                <ChartNoAxesCombined size={15} />
              ) : (
                <MonitorCog size={15} />
              )}
              {tab.title}
              {tab.profileId && (
                <X
                  size={14}
                  onClick={(event) => {
                    event.stopPropagation();
                    closeSession(tab.id);
                  }}
                />
              )}
            </button>
          ))}
        </nav>
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
    <section className="home-view">
      <div className="welcome">
        <div>
          <p>REMOTE FILE WORKSPACE</p>
          <h1>원격 서버 파일을 GUI로 관리하세요.</h1>
          <span>서버를 선택한 뒤 SFTP 폴더 브라우저를 엽니다.</span>
        </div>
      </div>
      <div className="card quick-list">
        <header>
          <h2>빠른 연결</h2>
        </header>
        {profiles.map((profile) => (
          <button
            className="quick-server"
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
      </div>
    </section>
  );
}

function parseTracing(content: string): TracingRecord[] {
  // eslint-disable-next-line no-control-regex -- ANSI SGR sequences from tracing output.
  const ansi = /\u001b\[[0-9;]*m/g;
  return content.split("\n").flatMap((raw) => {
    const line = raw.replace(ansi, "");
    const match = line.match(
      /^(\d{4}-\d{2}-\d{2}T\S+)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+(.+?):\s+(.*)$/,
    );
    if (!match) return [];
    return [
      {
        timestamp: match[1],
        level: match[2],
        target: match[3],
        message: match[4],
      },
    ];
  });
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
        <button
          className="primary"
          disabled={!profile || scanning}
          onClick={() => void loadFiles()}
        >
          파일 찾기
        </button>
      </section>
      <div className="analysis-layout">
        <section className="analysis-files card">
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
          <button
            className="primary analyze-button"
            disabled={!profile || selected.length === 0 || scanning}
            onClick={() => void analyze()}
          >
            {scanning ? "분석 중…" : "선택 파일 분석"}
          </button>
          <small className="analysis-limit">
            각 파일의 최신 5,000줄만 읽습니다.
          </small>
        </section>
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
          <section className="card source-table">
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
          </section>
          {visibleRecords.length > 0 && (
            <section className="card sample-table">
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
            </section>
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
            <p>NEW SFTP SESSION</p>
            <h2>원격 서버 추가</h2>
          </div>
          <button type="button" onClick={close}>
            <X size={18} />
          </button>
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
          <button className="primary">저장하고 열기</button>
        </footer>
      </form>
    </div>
  );
}
const root = document.getElementById("root");
if (!root) throw new Error("#root를 찾지 못했습니다.");
createRoot(root).render(<App />);
