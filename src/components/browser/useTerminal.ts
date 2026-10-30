import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { Profile, TerminalChunk } from "./types";

const initialOutput = "원격 터미널에 연결하는 중…\n";

export function useTerminal(profile: Profile, shouldConnect: boolean) {
  const [terminalId, setTerminalId] = useState<number | null>(null);
  const [output, setOutput] = useState("");
  const [starting, setStarting] = useState(false);
  const terminalIdRef = useRef<number | null>(null);
  const bufferedOutput = useRef(new Map<number, string>());

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<TerminalChunk>("terminal-chunk", (event) => {
      if (event.payload.terminalId !== terminalIdRef.current) {
        const previous =
          bufferedOutput.current.get(event.payload.terminalId) ?? "";
        bufferedOutput.current.set(
          event.payload.terminalId,
          `${previous}${event.payload.chunk}`.slice(-100_000),
        );
        return;
      }
      setOutput((current) =>
        `${current}${event.payload.chunk}`.slice(-100_000),
      );
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (!shouldConnect || terminalId !== null || starting) return;
    setStarting(true);
    setOutput(initialOutput);
    void invoke<number>("start_terminal", { profile })
      .then((id) => {
        terminalIdRef.current = id;
        setTerminalId(id);
        const buffered = bufferedOutput.current.get(id);
        if (buffered !== undefined) {
          bufferedOutput.current.delete(id);
          setOutput(`${initialOutput}${buffered}`);
        }
      })
      .catch((error) =>
        setOutput(`터미널을 열지 못했습니다.\n${String(error)}\n`),
      )
      .finally(() => setStarting(false));
  }, [profile, shouldConnect, starting, terminalId]);

  useEffect(
    () => () => {
      if (terminalIdRef.current !== null)
        void invoke("stop_terminal", { terminalId: terminalIdRef.current });
    },
    [],
  );

  async function write(input: string) {
    if (terminalId === null || !input) return;
    try {
      await invoke("write_terminal", { terminalId, input });
    } catch (error) {
      setOutput((current) => `${current}입력 전송 실패: ${String(error)}\n`);
    }
  }

  function clear() {
    setOutput("");
  }

  function resize(columns: number, rows: number) {
    if (terminalId === null) return;
    void invoke("resize_terminal", { terminalId, columns, rows });
  }

  return { clear, output, resize, starting, terminalId, write };
}
