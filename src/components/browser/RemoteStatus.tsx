import { invoke } from "@tauri-apps/api/core";
import { Cpu, HardDrive } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { Profile, RemoteSystemStatus } from "./types";

const REFRESH_INTERVAL_MS = 2_000;
const CPU_HISTORY_SIZE = 30;

function CpuHistoryGraph({ values }: { values: number[] }) {
  const width = 76;
  const height = 18;
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
      className="h-[18px] w-[76px] overflow-visible"
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
          strokeWidth="1.5"
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
  const requestInFlight = useRef(false);

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    const refresh = () => {
      if (requestInFlight.current) return;
      requestInFlight.current = true;
      void invoke<RemoteSystemStatus>("get_remote_system_status", { profile })
        .then((next) => {
          if (!cancelled) {
            setStatus(next);
            setCpuHistory((values) =>
              [...values, next.cpuPercent].slice(-CPU_HISTORY_SIZE),
            );
            setUnavailable(false);
          }
        })
        .catch(() => {
          if (!cancelled) setUnavailable(true);
        })
        .finally(() => {
          requestInFlight.current = false;
        });
    };
    refresh();
    const intervalId = window.setInterval(refresh, REFRESH_INTERVAL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [active, profile]);

  return (
    <footer className="flex h-7 shrink-0 items-center justify-end gap-3 border-t border-[#293846] bg-[#14202a] px-3 font-mono text-[10px] text-[#a7bac8]">
      {unavailable && !status ? (
        <span>서버 상태를 읽을 수 없음</span>
      ) : (
        <>
          <span
            className="flex items-center gap-1.5"
            title="최근 1분간 원격 서버 CPU 사용률 추이"
          >
            <Cpu size={13} className="text-[#78b8ff]" />
            CPU {status ? `${status.cpuPercent.toFixed(0)}%` : "…"}
            <CpuHistoryGraph values={cpuHistory} />
          </span>
          <span
            className="flex items-center gap-1"
            title="원격 서버 루트 파일시스템 사용량"
          >
            <HardDrive size={13} className="text-[#e7bc66]" />
            {status
              ? `DISK ${formatStorage(status.diskUsedKb)} / ${formatStorage(status.diskTotalKb)} (${status.diskPercent.toFixed(0)}%)`
              : "DISK …"}
          </span>
        </>
      )}
    </footer>
  );
}
