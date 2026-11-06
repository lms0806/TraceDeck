import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { Profile, TerminalChunk } from "./types";

const initialOutput = "원격 터미널에 연결하는 중…\n";
const maxOutputLength = 100_000;

export interface TerminalState {
  clear: () => void;
  output: string;
  reconnect: () => void;
  resize: (columns: number, rows: number) => void;
  starting: boolean;
  terminalId: number | null;
  write: (input: string) => Promise<void>;
}

export interface TerminalTab {
  key: number;
  terminalId: number | null;
  output: string;
  starting: boolean;
}

export function useTerminalTabs(profile: Profile, shouldConnect: boolean) {
  const [sessions, setSessions] = useState<TerminalTab[]>([]);
  const [activeKey, setActiveKey] = useState<number | null>(null);
  const sessionsRef = useRef<TerminalTab[]>([]);
  const bufferedOutput = useRef(new Map<number, string>());
  const terminalKeys = useRef(new Map<number, number>());
  const nextKey = useRef(1);
  const previouslyVisible = useRef(false);

  useEffect(() => {
    sessionsRef.current = sessions;
  }, [sessions]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<TerminalChunk>("terminal-chunk", (event) => {
      const key = terminalKeys.current.get(event.payload.terminalId);
      if (key === undefined) {
        const previous =
          bufferedOutput.current.get(event.payload.terminalId) ?? "";
        bufferedOutput.current.set(
          event.payload.terminalId,
          `${previous}${event.payload.chunk}`.slice(-maxOutputLength),
        );
        return;
      }
      setSessions((current) =>
        current.map((session) =>
          session.key === key
            ? {
                ...session,
                output: `${session.output}${event.payload.chunk}`.slice(
                  -maxOutputLength,
                ),
              }
            : session,
        ),
      );
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  }, []);

  const attachTerminal = useCallback((key: number, terminalId: number) => {
    terminalKeys.current.set(terminalId, key);
    const buffered = bufferedOutput.current.get(terminalId);
    if (buffered !== undefined) bufferedOutput.current.delete(terminalId);
    setSessions((current) =>
      current.map((session) =>
        session.key === key
          ? {
              ...session,
              terminalId,
              output:
                buffered === undefined
                  ? session.output
                  : `${session.output}${buffered}`.slice(-maxOutputLength),
              starting: false,
            }
          : session,
      ),
    );
  }, []);

  const addTerminal = useCallback(() => {
    const key = nextKey.current++;
    setSessions((current) => [
      ...current,
      { key, terminalId: null, output: initialOutput, starting: true },
    ]);
    setActiveKey(key);
    void invoke<number>("start_terminal", { profile })
      .then((terminalId) => attachTerminal(key, terminalId))
      .catch((error) => {
        setSessions((current) =>
          current.map((session) =>
            session.key === key
              ? {
                  ...session,
                  output: `터미널을 열지 못했습니다.\n${String(error)}\n`,
                  starting: false,
                }
              : session,
          ),
        );
      });
  }, [attachTerminal, profile]);

  useEffect(() => {
    if (
      shouldConnect &&
      !previouslyVisible.current &&
      sessionsRef.current.length === 0
    )
      addTerminal();
    previouslyVisible.current = shouldConnect;
  }, [addTerminal, shouldConnect]);

  useEffect(
    () => () => {
      for (const terminalId of terminalKeys.current.keys())
        void invoke("stop_terminal", { terminalId });
    },
    [],
  );

  const updateSession = useCallback(
    (key: number, update: (session: TerminalTab) => TerminalTab) => {
      setSessions((current) =>
        current.map((session) =>
          session.key === key ? update(session) : session,
        ),
      );
    },
    [],
  );

  const closeTerminal = useCallback((key: number) => {
    const removed = sessionsRef.current.find((session) => session.key === key);
    setSessions((current) => current.filter((session) => session.key !== key));
    setActiveKey((active) =>
      active === key
        ? (sessionsRef.current.filter((session) => session.key !== key).at(-1)
            ?.key ?? null)
        : active,
    );
    if (removed?.terminalId !== null && removed !== undefined) {
      terminalKeys.current.delete(removed.terminalId);
      void invoke("stop_terminal", { terminalId: removed.terminalId });
    }
  }, []);

  const terminalFor = useCallback(
    (key: number): TerminalState | undefined => {
      const session = sessions.find((item) => item.key === key);
      if (session === undefined) return undefined;
      return {
        terminalId: session.terminalId,
        output: session.output,
        starting: session.starting,
        clear: () =>
          updateSession(key, (current) => ({ ...current, output: "" })),
        reconnect: () => {
          const terminalId = sessionsRef.current.find(
            (item) => item.key === key,
          )?.terminalId;
          if (terminalId !== null && terminalId !== undefined) {
            terminalKeys.current.delete(terminalId);
            void invoke("stop_terminal", { terminalId });
          }
          updateSession(key, (current) => ({
            ...current,
            terminalId: null,
            output: initialOutput,
            starting: true,
          }));
          void invoke<number>("start_terminal", { profile })
            .then((terminalId) => attachTerminal(key, terminalId))
            .catch((error) =>
              updateSession(key, (current) => ({
                ...current,
                output: `터미널을 열지 못했습니다.\n${String(error)}\n`,
                starting: false,
              })),
            );
        },
        resize: (columns, rows) => {
          if (session.terminalId !== null)
            void invoke("resize_terminal", {
              terminalId: session.terminalId,
              columns,
              rows,
            });
        },
        write: async (input) => {
          if (session.terminalId === null || !input) return;
          try {
            await invoke("write_terminal", {
              terminalId: session.terminalId,
              input,
            });
          } catch (error) {
            updateSession(key, (current) => ({
              ...current,
              output: `${current.output}입력 전송 실패: ${String(error)}\n`,
            }));
          }
        },
      };
    },
    [attachTerminal, profile, sessions, updateSession],
  );

  return {
    activeKey,
    addTerminal,
    closeTerminal,
    sessions,
    setActiveKey,
    terminalFor,
  };
}
