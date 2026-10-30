import { useState } from "react";
import {
  ChartNoAxesCombined,
  FolderOpen,
  MonitorCog,
  Plus,
  Search,
  Server,
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
    <aside className="row-start-2 flex min-h-0 flex-col border-r border-[#293846] bg-[#151e28] px-2.5 py-[14px]">
      <label className="flex h-8 items-center gap-[7px] rounded border border-[#3a4d60] px-2 text-[#8fa5b8]">
        <Search size={15} />
        <input
          className="min-w-0 w-full border-0 bg-transparent text-xs text-[#e6eef6] outline-none"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="서버 검색"
        />
      </label>
      <div className="mx-2 mb-2 mt-5 flex items-center justify-between text-[10px] font-[750] tracking-[1px] text-[#7790a5]">
        <span>SAVED SESSIONS</span>
        <button
          className="border-0 bg-transparent p-px text-[#92a9ba]"
          onClick={onNewSession}
        >
          <Plus size={15} />
        </button>
      </div>
      <div className="overflow-auto">
        {shown.map((profile) => (
          <div className="group flex items-center rounded" key={profile.id}>
            <button
              className={
                selectedId === profile.id
                  ? "flex min-w-0 flex-1 items-center gap-[7px] rounded bg-[#1d403c] px-[7px] py-2 text-left text-[#c1d0dd] shadow-[inset_2px_0_#52d4a2]"
                  : "flex min-w-0 flex-1 items-center gap-[7px] rounded bg-transparent px-[7px] py-2 text-left text-[#c1d0dd] hover:bg-[#202d39]"
              }
              onClick={() => onOpenProfile(profile)}
            >
              <span className="h-[7px] w-[7px] flex-none rounded-full bg-[#66788b]" />
              <Server size={15} />
              <span className="grid min-w-0 gap-0.5">
                <strong>{profile.name}</strong>
                <small className="overflow-hidden text-[10px] text-ellipsis text-[#7e95a8]">
                  {profile.username}@{profile.host}
                </small>
              </span>
              <span className="ml-auto text-[9px] text-[#7790a5]">SFTP</span>
            </button>
            <button
              className="invisible border-0 bg-transparent p-1.5 text-[#8ca1b1] group-hover:visible group-focus-within:visible hover:rounded hover:bg-[#41272a] hover:text-[#ff9b94]"
              onClick={() => onRemoveProfile(profile)}
              aria-label={`${profile.name} 저장 세션 삭제`}
            >
              <X size={14} />
            </button>
          </div>
        ))}
      </div>
      <div className="mx-2 mb-2 mt-4 flex items-center justify-between text-[10px] font-[750] tracking-[1px] text-[#7790a5]">
        <span>OPEN SESSIONS</span>
      </div>
      <div className="grid max-h-[130px] gap-0.5 overflow-auto">
        {tabs
          .filter((tab) => tab.profileId !== undefined)
          .map((tab) => (
            <div
              className="flex items-center rounded hover:bg-[#202d39]"
              key={tab.id}
            >
              <button
                className="flex min-w-0 flex-1 items-center gap-1.5 overflow-hidden text-ellipsis whitespace-nowrap border-0 bg-transparent p-[7px] text-left text-[11px] text-[#a9bfce]"
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
                className="border-0 bg-transparent p-1.5 text-[11px] text-[#8aa1b2]"
                onClick={() => onCloseSession(tab.id)}
                aria-label={`${tab.title} 세션 닫기`}
              >
                <X size={14} />
              </button>
            </div>
          ))}
      </div>
      <div className="mt-auto grid gap-0.5 border-t border-[#293846] pt-[9px]">
        <button
          className="flex gap-2 rounded border-0 bg-transparent p-2 text-left text-xs text-[#92a6b8] hover:bg-[#202d39]"
          onClick={onHome}
        >
          <MonitorCog size={16} /> 홈
        </button>
        <button
          className="flex gap-2 rounded border-0 bg-transparent p-2 text-left text-xs text-[#92a6b8] hover:bg-[#202d39]"
          onClick={onAnalysis}
        >
          <ChartNoAxesCombined size={16} /> 분석
        </button>
      </div>
    </aside>
  );
}
