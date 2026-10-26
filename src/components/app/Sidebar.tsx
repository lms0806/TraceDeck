import { useState } from "react";
import {
  ChartNoAxesCombined,
  FolderOpen,
  MonitorCog,
  Plus,
  Search,
  Server,
  ShieldCheck,
  X,
} from "lucide-react";

import type { Profile, Tab } from "../../app/types";

export function Sidebar({
  profiles,
  selectedId,
  tabs,
  onOpenProfile,
  onRemoveProfile,
  onCloseSession,
  onNewSession,
  onHome,
  onAnalysis,
}: {
  profiles: Profile[];
  selectedId: string;
  tabs: Tab[];
  onOpenProfile: (profile: Profile) => void;
  onRemoveProfile: (profile: Profile) => void;
  onCloseSession: (id: string) => void;
  onNewSession: () => void;
  onHome: () => void;
  onAnalysis: () => void;
}) {
  const [query, setQuery] = useState("");
  const shown = profiles.filter((profile) =>
    `${profile.name} ${profile.host} ${profile.group}`
      .toLowerCase()
      .includes(query.toLowerCase()),
  );
  return (
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
        <button onClick={onNewSession}>
          <Plus size={15} />
        </button>
      </div>
      <div className="session-tree">
        {shown.map((profile) => (
          <div className="saved-session" key={profile.id}>
            <button
              className={
                selectedId === profile.id ? "server-row selected" : "server-row"
              }
              onClick={() => onOpenProfile(profile)}
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
              onClick={() => onRemoveProfile(profile)}
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
              <button
                onClick={() =>
                  onOpenProfile(
                    profiles.find((profile) => profile.id === tab.profileId)!,
                  )
                }
              >
                <FolderOpen size={14} />
                {tab.title}
              </button>
              <button
                className="close-session"
                onClick={() => onCloseSession(tab.id)}
                aria-label={`${tab.title} 세션 닫기`}
              >
                <X size={14} />
              </button>
            </div>
          ))}
      </div>
      <div className="sidebar-footer">
        <button onClick={onHome}>
          <MonitorCog size={16} /> 홈
        </button>
        <button onClick={onAnalysis}>
          <ChartNoAxesCombined size={16} /> 분석
        </button>
        <button>
          <ShieldCheck size={16} /> 보안 상태
        </button>
      </div>
    </aside>
  );
}
