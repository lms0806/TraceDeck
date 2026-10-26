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
    <nav className="tabs">
      {tabs.map((tab) => (
        <button
          className={
            tab.id === activeId
              ? `tab active${draggedId === tab.id ? " dragging" : ""}`
              : `tab${draggedId === tab.id ? " dragging" : ""}`
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
