import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Cpu, HardDrive } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { Profile, RemoteSystemStatus, SystemStatusChunk } from "./types";

const CPU_HISTORY_SIZE = 30;

function CpuHistoryGraph({ values }: { values: number[] }) {
  const width = 96;
  const height = 24;
  const points = values
    .map((value, index) => {
      const x = (index / Math.max(values.length - 1, 1)) * width;
      const y = height - (Math.min(Math.max(value, 0), 100) / 100) * height;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");

  return (
    <svg
      aria-label="최근 CPU 사용률 추이"
      className="h-6 w-24 overflow-visible"
      viewBox={`0 0 ${width} ${height}`}
    >
      <path
        d={`M0 ${height / 2} H${width}`}
        stroke="#405968"
        strokeDasharray="2 2"
        strokeWidth="1"
      />
      {points && (
        <polyline
          fill="none"
          points={points}
          stroke="#78b8ff"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="2"
        />
      )}
    </svg>
  );
}

function formatStorage(kilobytes: number) {
  const units = ["KB", "MB", "GB", "TB"];
  let value = kilobytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value >= 10 || unitIndex === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`;
}

export function RemoteStatus({
  profile,
  active,
}: {
  profile: Profile;
  active: boolean;
}) {
  const [status, setStatus] = useState<RemoteSystemStatus | null>(null);
  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [unavailable, setUnavailable] = useState(false);
  const streamIdRef = useRef<number | null>(null);
  const bufferedStatus = useRef(new Map<number, RemoteSystemStatus>());

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<SystemStatusChunk>("system-status-chunk", (event) => {
      const { streamId, status: next } = event.payload;
      if (streamId !== streamIdRef.current) {
        bufferedStatus.current.set(streamId, next);
        return;
      }
      setStatus(next);
      setCpuHistory((values) =>
        [...values, next.cpuPercent].slice(-CPU_HISTORY_SIZE),
      );
      setUnavailable(false);
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    void invoke<number>("start_remote_system_status_stream", { profile })
      .then((streamId) => {
        if (cancelled) {
          void invoke("stop_remote_system_status_stream", { streamId });
          return;
        }
        streamIdRef.current = streamId;
        setStatus(null);
        setCpuHistory([]);
        setUnavailable(false);
        const buffered = bufferedStatus.current.get(streamId);
        if (buffered) {
          bufferedStatus.current.delete(streamId);
          setStatus(buffered);
          setCpuHistory([buffered.cpuPercent]);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setStatus(null);
          setUnavailable(true);
        }
      });
    return () => {
      cancelled = true;
      const streamId = streamIdRef.current;
      streamIdRef.current = null;
      if (streamId !== null)
        void invoke("stop_remote_system_status_stream", { streamId });
    };
  }, [active, profile]);

  return (
    <footer className="flex h-10 shrink-0 items-center justify-start gap-4 border-t border-[#293846] bg-[#14202a] px-4 font-mono text-xs text-[#c2d2dd]">
      {unavailable && !status ? (
        <span>서버 상태를 읽을 수 없음</span>
      ) : (
        <>
          <span
            className="flex items-center gap-1.5"
            title="최근 1분간 원격 서버 CPU 사용률 추이"
          >
            <Cpu size={16} className="text-[#78b8ff]" />
            CPU {status ? `${status.cpuPercent.toFixed(0)}%` : "…"}
            <CpuHistoryGraph values={cpuHistory} />
          </span>
          <span
            className="flex items-center gap-1"
            title="원격 서버 루트 파일시스템 사용량"
          >
            <HardDrive size={16} className="text-[#e7bc66]" />
            {status
              ? `DISK ${formatStorage(status.diskUsedKb)} / ${formatStorage(status.diskTotalKb)} (${status.diskPercent.toFixed(0)}%)`
              : "DISK …"}
          </span>
        </>
      )}
    </footer>
  );
}
