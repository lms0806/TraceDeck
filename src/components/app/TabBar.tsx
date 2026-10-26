import { useRef, useState } from "react";
import { ChartNoAxesCombined, FolderOpen, MonitorCog, X } from "lucide-react";

import type { Tab } from "../../app/types";

export function TabBar({
  tabs,
  activeId,
  onActivate,
  onClose,
  onMove,
}: {
  tabs: Tab[];
  activeId: string;
  onActivate: (id: string) => void;
  onClose: (id: string) => void;
  onMove: (draggedId: string, targetId: string, insertAfter: boolean) => void;
}) {
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const drag = useRef<{ id: string; startX: number } | null>(null);
  function endDrag() {
    drag.current = null;
    setDraggedId(null);
  }
  function updateDrag(clientX: number, clientY: number) {
    if (!drag.current || Math.abs(clientX - drag.current.startX) < 6) return;
    setDraggedId(drag.current.id);
    const target = document
      .elementFromPoint(clientX, clientY)
      ?.closest<HTMLButtonElement>("[data-tab-id]");
    if (!target?.dataset.tabId) return;
    const bounds = target.getBoundingClientRect();
    onMove(
      drag.current.id,
      target.dataset.tabId,
      clientX > bounds.left + bounds.width / 2,
    );
  }
  return (
    <nav className="flex h-[42px] overflow-auto border-b border-[#293846] bg-[#151e28]">
      {tabs.map((tab) => (
        <button
          className={
            tab.id === activeId
              ? `flex min-w-[145px] items-center gap-[7px] border-0 border-r border-[#293846] bg-[#111820] px-[13px] text-[#f2f7fb] shadow-[inset_0_2px_#50d3a1] ${draggedId === tab.id ? "cursor-grabbing opacity-45" : tab.id !== "home" ? "cursor-grab active:cursor-grabbing" : ""}`
              : `flex min-w-[145px] items-center gap-[7px] border-0 border-r border-[#293846] bg-[#151e28] px-[13px] text-[#91a7b9] ${draggedId === tab.id ? "cursor-grabbing opacity-45" : tab.id !== "home" ? "cursor-grab active:cursor-grabbing" : ""}`
          }
          key={tab.id}
          onClick={() => onActivate(tab.id)}
          data-tab-id={tab.id}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            if (tab.id !== "home")
              drag.current = { id: tab.id, startX: event.clientX };
          }}
          onPointerMove={(event) => updateDrag(event.clientX, event.clientY)}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
        >
          {tab.profileId ? (
            <FolderOpen size={15} />
          ) : tab.id === "analysis" ? (
            <ChartNoAxesCombined size={15} />
          ) : (
            <MonitorCog size={15} />
          )}
          {tab.title}
          {tab.id !== "home" && (
            <X
              className="ml-auto"
              size={14}
              onClick={(event) => {
                event.stopPropagation();
                onClose(tab.id);
              }}
            />
          )}
        </button>
      ))}
    </nav>
  );
}
